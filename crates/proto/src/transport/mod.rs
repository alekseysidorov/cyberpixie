pub use packet::PacketKind;

mod packet;

// TODO Handle events like CONNECT, DISCONNECT, CONNECT FAIL.

pub trait Transport {
    type Error;
    type Address: PartialEq + Clone + Copy;
    type Payload: AsRef<[u8]>;

    fn poll_next_packet(&mut self)
        -> nb::Result<Packet<Self::Address, Self::Payload>, Self::Error>;

    fn confirm_packet(&mut self, from: Self::Address) -> Result<(), Self::Error>;

    fn send_packet<P: AsRef<[u8]>>(
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
