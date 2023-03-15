use core::{array::IntoIter, fmt::Debug};

use nb_utils::NbResultExt;
pub use packet::PacketKind;

mod packet;

// TODO Handle events like CONNECT, DISCONNECT, CONNECT FAIL.

pub trait Transport {
    type Error: Debug;
    type Address: PartialEq + Clone + Copy;
    type Payload: AsRef<[u8]>;

    fn poll_next_event(&mut self) -> nb::Result<Event<Self::Address, Self::Payload>, Self::Error>;

    fn confirm_packet(&mut self, from: Self::Address) -> Result<(), Self::Error>;

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        to: Self::Address,
    ) -> Result<(), Self::Error>;

    fn wait_for_confirmation(&mut self, from: Self::Address) -> Result<(), Self::Error> {
        nb::block!(self
            .poll_next_event()
            .filter(|event| event.address() == &from)
            .filter_map(Event::packet)
            .filter_map(PacketData::confirmed))
    }

    fn wait_for_payload(&mut self, from: Self::Address) -> Result<Self::Payload, Self::Error> {
        nb::block!(self
            .poll_next_event()
            .filter(|event| event.address() == &from)
            .filter_map(Event::packet)
            .filter_map(PacketData::payload))
    }
}

#[derive(Debug, PartialEq)]
pub enum Event<A, P> {
    Connected { address: A },
    Disconnected { address: A },
    Packet { address: A, data: PacketData<P> },
}

impl<A, P> Event<A, P> {
    pub fn address(&self) -> &A {
        match self {
            Event::Connected { address } => address,
            Event::Disconnected { address } => address,
            Event::Packet { address, .. } => address,
        }
    }

    pub fn packet(self) -> Option<PacketData<P>> {
        if let Self::Packet { data, .. } = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn connected(self) -> Option<A> {
        if let Self::Connected { address } = self {
            Some(address)
        } else {
            None
        }
    }

    pub fn disconnected(self) -> Option<A> {
        if let Self::Disconnected { address } = self {
            Some(address)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PacketData<P> {
    Payload(P),
    Confirmed,
}

impl<P> PacketData<P> {
    pub fn payload(self) -> Option<P> {
        if let PacketData::Payload(payload) = self {
            Some(payload)
        } else {
            None
        }
    }

    pub fn confirmed(self) -> Option<()> {
        if let PacketData::Confirmed = self {
            Some(())
        } else {
            None
        }
    }
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

    pub fn payload_len(&self) -> usize {
        self.payload.len()
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
