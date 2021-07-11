use cyberpixie::{
    nb_utils::NbResultExt,
    proto::{PacketData, PacketKind, PacketWithPayload, Transport, TransportEvent},
};
use embedded_hal::serial::{Read, Write};
use esp8266_wifi_serial::{
    clock::{Deadline, SimpleClock},
    Error as WifiError, Event as SocketEvent, WifiSession,
};
use heapless::Vec;

use crate::config::ADAPTER_BUF_CAPACITY;

const MAX_PAYLOAD_LEN: usize = ADAPTER_BUF_CAPACITY - PacketKind::PACKED_LEN;

type TransportImplEvent = TransportEvent<usize, Vec<u8, MAX_PAYLOAD_LEN>>;

macro_rules! block_deadline {
    ($deadline:expr, $e:expr) => {
        loop {
            #[allow(unreachable_patterns)]
            match $e {
                Err(nb::Error::Other(e)) =>
                {
                    #[allow(unreachable_code)]
                    break Err(e)
                }
                Err(nb::Error::WouldBlock) => {}
                Ok(x) => break Ok(x),
            }

            $deadline.reached().map_err(|_| WifiError::Timeout)?;
        }
    };
}

pub struct TransportImpl<Tx, Rx, C, const N: usize>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,
{
    session: WifiSession<Rx, Tx, C, N>,
    clock: C,
}

impl<'a, Tx, Rx, C, const N: usize> TransportImpl<Tx, Rx, C, N>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,
{
    pub fn new(session: WifiSession<Rx, Tx, C, N>, clock: C) -> Self {
        Self { session, clock }
    }

    fn poll_next_event(
        session: &'a mut WifiSession<Rx, Tx, C, N>,
    ) -> nb::Result<TransportImplEvent, WifiError> {
        let event = session.poll_next_event()?;

        Ok(match event {
            SocketEvent::Connected { link_id } => TransportEvent::Connected { address: link_id },
            SocketEvent::Closed { link_id } => TransportEvent::Disconnected { address: link_id },
            SocketEvent::DataAvailable { link_id, data } => {
                let mut reader = data.iter().copied();
                let packet = match PacketKind::from_reader(reader.by_ref()) {
                    PacketKind::Payload(len) => {
                        assert_eq!(len, reader.len());
                        let mut payload: Vec<u8, MAX_PAYLOAD_LEN> = Vec::new();
                        payload.extend(reader.by_ref());
                        PacketData::Payload(payload)
                    }
                    PacketKind::Confirmed => PacketData::Confirmed,
                };

                TransportEvent::Packet {
                    address: link_id,
                    data: packet,
                }
            }
        })
    }
}

impl<Tx, Rx, C, const N: usize> Transport for TransportImpl<Tx, Rx, C, N>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,
{
    type Error = WifiError;
    type Address = usize;
    type Payload = Vec<u8, MAX_PAYLOAD_LEN>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<TransportEvent<Self::Address, Self::Payload>, Self::Error> {
        Self::poll_next_event(&mut self.session)
    }

    fn confirm_packet(&mut self, address: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::Confirmed.to_bytes();

        let bytes = packet.iter().copied();
        self.session.send_to(address, bytes)
    }

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        address: Self::Address,
    ) -> Result<(), Self::Error> {
        assert!(payload.len() <= MAX_PAYLOAD_LEN);
        self.session
            .send_to(address, PacketWithPayload::from(payload))
    }

    fn wait_for_confirmation(&mut self, from: Self::Address) -> Result<(), Self::Error> {
        let deadline = Deadline::new(&self.clock, self.session.socket_timeout());

        block_deadline!(
            deadline,
            Self::poll_next_event(&mut self.session)
                .filter(|event| event.address() == &from)
                .filter_map(TransportEvent::packet)
                .filter_map(PacketData::confirmed)
        )
    }

    fn wait_for_payload(&mut self, from: Self::Address) -> Result<Self::Payload, Self::Error> {
        let deadline = Deadline::new(&self.clock, self.session.socket_timeout());

        block_deadline!(
            deadline,
            Self::poll_next_event(&mut self.session)
                .filter(|event| event.address() == &from)
                .filter_map(TransportEvent::packet)
                .filter_map(PacketData::payload)
        )
    }
}
