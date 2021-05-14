use crate::Message;

// Service trait types relied on the associated types and thus cannot be simplified
// by the type alias
#[allow(clippy::type_complexity)]

pub trait Service {
    type Error: core::fmt::Debug;

    type Address;
    type BytesReader<'a>: Iterator<Item = u8> + ExactSizeIterator + 'a;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<ServiceEvent<Self::Address, Self::BytesReader<'_>>, Self::Error>;

    fn send_message<I>(
        &mut self,
        to: Self::Address,
        message: Message<I>,
    ) -> Result<(), Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator;

    fn poll_next_message(
        &mut self,
    ) -> nb::Result<(Self::Address, Message<Self::BytesReader<'_>>), Self::Error> {
        let event = self.poll_next_event()?;
        if let ServiceEvent::Data { address, payload } = event {
            Ok((address, payload))
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

#[derive(Debug)]
pub enum ServiceEvent<A, I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    Connected { address: A },
    Disconnected { address: A },
    Data { address: A, payload: Message<I> },
}
