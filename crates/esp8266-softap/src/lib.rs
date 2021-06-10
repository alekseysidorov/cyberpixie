#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::{Adapter, ReadPart, WritePart},
    bytes_iter::BytesIter,
    error::{Error, Result},
    softap::{DataReader, Event, SoftApConfig, TcpSocket},
};

pub mod adapter;
pub mod error;
pub mod softap;

mod bytes_iter;
mod parser;
#[cfg(test)]
mod tests;

pub const ADAPTER_BUF_CAPACITY: usize = 512;
