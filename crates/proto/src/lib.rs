#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use crate::packet::{
    write_message_header, FirmwareInfo, Message, PacketReader, MAX_HEADER_LEN,
};
pub use postcard::Error as PayloadError;

pub mod types;

mod packet;
#[cfg(all(test, not(target_os = "none")))]
mod tests;

// TODO: Think about the `link_id` handling.
pub trait Service {
    type Error: core::fmt::Debug;

    type Address;
    type BytesReader<'a>: Iterator<Item = u8> + ExactSizeIterator + 'a;

    fn poll_next(
        &mut self,
    ) -> nb::Result<ServiceEvent<Self::Address, Self::BytesReader<'_>>, Self::Error>;

    fn send_message<I>(
        &mut self,
        to: Self::Address,
        message: Message<I>,
    ) -> Result<(), Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator;
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
