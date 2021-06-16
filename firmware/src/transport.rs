use core::fmt::Debug;

use cyberpixie::proto::{PacketData, PacketKind, PacketWithPayload, Transport, TransportEvent};
use embedded_hal::serial::{Read, Write};
use esp8266_softap::{Error as SocketError, Event as SoftApEvent, TcpStream, ADAPTER_BUF_CAPACITY};
use heapless::Vec;
use stdio_serial::uprintln;

const MAX_PAYLOAD_LEN: usize = ADAPTER_BUF_CAPACITY - PacketKind::PACKED_LEN;

pub struct TransportImpl<Tx, Rx>(TcpStream<Rx, Tx>)
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,

    Rx::Error: Debug,
    Tx::Error: Debug;

impl<Tx, Rx> TransportImpl<Tx, Rx>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    Rx::Error: Debug,
    Tx::Error: Debug,
{
    pub fn new(stream: TcpStream<Rx, Tx>) -> Self {
        Self(stream)
    }
}

impl<Tx, Rx> Transport for TransportImpl<Tx, Rx>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,

    Rx::Error: Debug,
    Tx::Error: Debug,
{
    type Error = SocketError<Rx::Error, Tx::Error>;
    type Address = usize;
    type Payload = Vec<u8, MAX_PAYLOAD_LEN>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<TransportEvent<Self::Address, Self::Payload>, Self::Error> {
        let event = self
            .0
            .poll_next_event()
            .map_err(|x| x.map(SocketError::Read))?;

        Ok(match event {
            SoftApEvent::Connected { link_id } => TransportEvent::Connected { address: link_id },
            SoftApEvent::Closed { link_id } => TransportEvent::Disconnected { address: link_id },
            SoftApEvent::DataAvailable {
                link_id,
                mut reader,
            } => {
                let data = match PacketKind::from_reader(reader.by_ref()) {
                    PacketKind::Payload(len) => {
                        assert_eq!(len, reader.len());
                        let mut payload: Vec<u8, MAX_PAYLOAD_LEN> = Vec::new();
                        payload.extend(reader.by_ref());

                        uprintln!("RM");
                        PacketData::Payload(payload)
                    }
                    PacketKind::Confirmed => {
                        uprintln!("RC");
                        PacketData::Confirmed
                    },
                };

                assert_eq!(reader.len(), 0);
                TransportEvent::Packet {
                    address: link_id,
                    data,
                }
            }
        })
    }

    fn confirm_packet(&mut self, address: Self::Address) -> Result<(), Self::Error> {
        uprintln!("Sending packet confirmation to {}", address);
        let packet = PacketKind::Confirmed.to_bytes();

        let bytes = packet.iter().copied();
        self.0.send_packet_to_link(address, bytes)
    }

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        address: Self::Address,
    ) -> Result<(), Self::Error> {
        uprintln!("Sending packet to {} with len {}", address, payload.len());

        assert!(payload.len() <= MAX_PAYLOAD_LEN);
        self.0
            .send_packet_to_link(address, PacketWithPayload::from(payload))
    }
}
