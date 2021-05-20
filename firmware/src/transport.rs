use core::fmt::Debug;

use cyberpixie::proto::transport::{Packet, PacketData, PacketKind, Transport};
use embedded_hal::serial::{Read, Write};
use esp8266_softap::{Error as SoftApError, SoftAp, ADAPTER_BUF_CAPACITY};
use heapless::Vec;
use stdio_serial::uprintln;

const MAX_PAYLOAD_LEN: usize = ADAPTER_BUF_CAPACITY - PacketKind::PACKED_LEN;

pub struct TransportImpl<Tx, Rx>(SoftAp<Rx, Tx>)
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
    Tx::Error: Debug
{
    pub fn new(ap: SoftAp<Rx, Tx>) -> Self {
        Self(ap)
    }
}

impl<Tx, Rx> Transport for TransportImpl<Tx, Rx>
where
    Rx: Read<u8> + 'static,
    Tx: Write<u8> + 'static,
    Rx::Error: Debug,
    Tx::Error: Debug,
{
    type Error = SoftApError<Rx::Error, Tx::Error>;
    type Address = usize;
    type Payload = Vec<u8, MAX_PAYLOAD_LEN>;

    fn poll_next_packet(
        &mut self,
    ) -> nb::Result<Packet<Self::Address, Self::Payload>, Self::Error> {
        let event = self
            .0
            .poll_next_event()
            .map_err(|x| x.map(SoftApError::Read))?;

        if let esp8266_softap::Event::DataAvailable {
            link_id,
            mut reader,
        } = event
        {
            let mut header = [0_u8; PacketKind::PACKED_LEN];
            for (idx, byte) in reader.by_ref().take(header.len()).enumerate() {
                header[idx] = byte;
            }
            let kind = PacketKind::decode(&header);
            let data = match kind {
                PacketKind::Payload(len) => {
                    assert_eq!(len, reader.len());
                    let mut payload: Vec<u8, MAX_PAYLOAD_LEN> = Vec::new();
                    payload.extend(reader);
                    PacketData::Payload(payload)
                }
                PacketKind::RequestNext => PacketData::RequestNext,
            };

            uprintln!("[Info] received packet from {}: {:?}", link_id, data);
            return Ok(Packet {
                address: link_id,
                data,
            });
        };

        Err(nb::Error::WouldBlock)
    }

    fn request_next_packet(&mut self, from: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::RequestNext.to_bytes();
        uprintln!("[Info] sending request next packet to {}", from);

        let bytes = packet.iter().copied();
        self.0.send_packet_to_link(from, bytes)
    }

    fn send_packet<P: AsRef<[u8]>>(
        &mut self,
        payload: P,
        to: Self::Address,
    ) -> Result<(), Self::Error> {
        // TODO remove extra copying.
        let mut packet: Vec<u8, MAX_PAYLOAD_LEN> = Vec::new();
        packet
            .extend_from_slice(
                PacketKind::Payload(payload.as_ref().len())
                    .to_bytes()
                    .as_ref(),
            )
            .unwrap();
        packet.extend_from_slice(payload.as_ref()).unwrap();

        let bytes = packet.as_slice().iter().copied();
        self.0.send_packet_to_link(to, bytes)
    }
}
