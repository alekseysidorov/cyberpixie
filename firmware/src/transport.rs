use core::fmt::Debug;

use cyberpixie::proto::{Packet, PacketData, PacketKind, Transport};
use embedded_hal::serial::{Read, Write};
use esp8266_softap::{Error as SoftApError, SoftAp, ADAPTER_BUF_CAPACITY};
use heapless::Vec;

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
    Tx::Error: Debug,
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
                    payload.extend(reader.by_ref());
                    PacketData::Payload(payload)
                }
                PacketKind::Confirmed => PacketData::Confirmed,
            };

            assert_eq!(reader.len(), 0);
            return Ok(Packet {
                address: link_id,
                data,
            });
        };

        Err(nb::Error::WouldBlock)
    }

    fn confirm_packet(&mut self, address: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::Confirmed.to_bytes();

        let bytes = packet.iter().copied();
        self.0.send_packet_to_link(address, bytes)
    }

    fn send_packet<P: AsRef<[u8]>>(
        &mut self,
        payload: P,
        address: Self::Address,
    ) -> Result<(), Self::Error> {
        assert!(payload.as_ref().len() <= MAX_PAYLOAD_LEN);
        self.0
            .send_packet_to_link(address, PacketWithPayload::new(payload))
    }
}

struct PacketWithPayload<P: AsRef<[u8]>> {
    header: [u8; PacketKind::PACKED_LEN],
    payload: P,
    pos: usize,
}

impl<P: AsRef<[u8]>> PacketWithPayload<P> {
    fn new(payload: P) -> Self {
        let header = PacketKind::Payload(payload.as_ref().len()).to_bytes();
        Self {
            header,
            payload,
            pos: 0,
        }
    }

    fn total_len(&self) -> usize {
        self.header.len() + self.payload.as_ref().len()
    }
}

impl<P: AsRef<[u8]>> Iterator for PacketWithPayload<P> {
    type Item = u8;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let bytes_remaining = self.total_len() - self.pos;
        (bytes_remaining, Some(bytes_remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.total_len() {
            return None;
        }

        let byte = if self.pos < self.header.len() {
            self.header[self.pos]
        } else {
            let pos = self.pos - self.header.len();
            self.payload.as_ref()[pos]
        };
        self.pos += 1;
        Some(byte)
    }
}

impl<P: AsRef<[u8]>> ExactSizeIterator for PacketWithPayload<P> {}
