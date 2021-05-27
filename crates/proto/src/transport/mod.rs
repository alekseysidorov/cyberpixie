pub use packet::PacketKind;

use core::array::IntoIter;

mod packet;

// TODO Handle events like CONNECT, DISCONNECT, CONNECT FAIL.

pub trait Transport {
    type Error;
    type Address: PartialEq + Clone + Copy;
    type Payload: AsRef<[u8]>;

    fn poll_next_packet(&mut self)
        -> nb::Result<Packet<Self::Address, Self::Payload>, Self::Error>;

    fn confirm_packet(&mut self, from: Self::Address) -> Result<(), Self::Error>;

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        to: Self::Address,
    ) -> Result<(), Self::Error>;

    fn poll_for_confirmation(&mut self, from: Self::Address) -> nb::Result<(), Self::Error> {
        let packet = nb::block!(self.poll_next_packet())?;
        if packet.address != from {
            return Err(nb::Error::WouldBlock);
        }

        if let PacketData::Confirmed = packet.data {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn poll_for_payload(&mut self, from: Self::Address) -> nb::Result<Self::Payload, Self::Error> {
        let packet = nb::block!(self.poll_next_packet())?;
        if packet.address != from {
            return Err(nb::Error::WouldBlock);
        }

        if let PacketData::Payload(payload) = packet.data {
            Ok(payload)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Packet<A, P> {
    pub address: A,
    pub data: PacketData<P>,
}

#[derive(Debug, PartialEq)]
pub enum PacketData<P> {
    Payload(P),
    Confirmed,
}

pub struct PacketWithPayload<P>
where
    P: Iterator<Item = u8> + ExactSizeIterator,
{
    header: IntoIter<u8, { PacketKind::PACKED_LEN }>,
    payload: P,
}

impl<P: Iterator<Item = u8> + ExactSizeIterator> PacketWithPayload<P> {
    pub fn new(payload: P) -> Self {
        let header = PacketKind::Payload(payload.len()).to_bytes();
        Self {
            header: IntoIter::new(header),
            payload,
        }
    }

    fn total_len(&self) -> usize {
        self.header.len() + self.payload.len()
    }
}

impl<P: Iterator<Item = u8> + ExactSizeIterator> Iterator for PacketWithPayload<P> {
    type Item = u8;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let bytes_remaining = self.total_len();
        (bytes_remaining, Some(bytes_remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.header.next() {
            return Some(byte);
        }
        self.payload.next()
    }
}

impl<P: Iterator<Item = u8> + ExactSizeIterator> ExactSizeIterator for PacketWithPayload<P> {}

impl<P: Iterator<Item = u8> + ExactSizeIterator> From<P> for PacketWithPayload<P> {
    fn from(payload: P) -> Self {
        Self::new(payload)
    }
}
