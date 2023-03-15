use core::fmt::Debug;

use cyberpixie::{
    nb_utils::NbResultExt,
    proto::{PacketData, PacketKind, PacketWithPayload, Transport, TransportEvent},
};
use embedded_hal::serial::{Read, Write};
use esp8266_softap::{
    clock::{Deadline, SimpleClock},
    Error as SocketError, Event as SocketEvent, TcpSocket, ADAPTER_BUF_CAPACITY,
};
use heapless::Vec;

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

            $deadline.reached().map_err(|_| SocketError::Timeout)?;
        }
    };
}

pub struct TransportImpl<Tx, Rx, C>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,

    Rx::Error: Debug,
    Tx::Error: Debug,
{
    socket: TcpSocket<Rx, Tx, C>,
    clock: C,
}

impl<Tx, Rx, C> TransportImpl<Tx, Rx, C>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,

    Rx::Error: Debug,
    Tx::Error: Debug,
{
    pub fn new(socket: TcpSocket<Rx, Tx, C>, clock: C) -> Self {
        Self { socket, clock }
    }

    fn poll_next_event(
        socket: &mut TcpSocket<Rx, Tx, C>,
    ) -> nb::Result<TransportImplEvent, SocketError<Rx::Error, Tx::Error>> {
        let event = socket
            .poll_next_event()
            .map_err(|x| x.map(SocketError::Read))?;

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

impl<Tx, Rx, C> Transport for TransportImpl<Tx, Rx, C>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    C: SimpleClock,

    Rx::Error: Debug,
    Tx::Error: Debug,
{
    type Error = SocketError<Rx::Error, Tx::Error>;
    type Address = usize;
    type Payload = Vec<u8, MAX_PAYLOAD_LEN>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<TransportEvent<Self::Address, Self::Payload>, Self::Error> {
        Self::poll_next_event(&mut self.socket)
    }

    fn confirm_packet(&mut self, address: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::Confirmed.to_bytes();

        let bytes = packet.iter().copied();
        self.socket.send_packet_to_link(address, bytes)
    }

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        address: Self::Address,
    ) -> Result<(), Self::Error> {
        assert!(payload.len() <= MAX_PAYLOAD_LEN);
        self.socket
            .send_packet_to_link(address, PacketWithPayload::from(payload))
    }

    fn wait_for_confirmation(&mut self, from: Self::Address) -> Result<(), Self::Error> {
        let deadline = Deadline::new(&self.clock, self.socket.socket_timeout());
        block_deadline!(
            deadline,
            Self::poll_next_event(&mut self.socket)
                .filter(|event| event.address() == &from)
                .filter_map(TransportEvent::packet)
                .filter_map(PacketData::confirmed)
        )
    }

    fn wait_for_payload(&mut self, from: Self::Address) -> Result<Self::Payload, Self::Error> {
        let deadline = Deadline::new(&self.clock, self.socket.socket_timeout());
        block_deadline!(
            deadline,
            Self::poll_next_event(&mut self.socket)
                .filter(|event| event.address() == &from)
                .filter_map(TransportEvent::packet)
                .filter_map(PacketData::payload)
        )
    }
}
