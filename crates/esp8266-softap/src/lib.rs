#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::{Adapter, ReadPart, WriterPart},
    bytes_iter::BytesIter,
    error::{Error, Result},
    softap::{DataReader, Event, SoftAp, SoftApConfig},
};

pub mod adapter;
pub mod error;
pub mod softap;

mod bytes_iter;
mod parser;
#[cfg(test)]
mod tests;

pub const ADAPTER_BUF_CAPACITY: usize = 512;

#[macro_export]
macro_rules! poll_continue {
    ($e:expr) => {
        match $e {
            Err(nb::Error::WouldBlock) => continue,
            other => other,
        }
    };
}
