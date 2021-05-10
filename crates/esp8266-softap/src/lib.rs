#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::{Adapter, ReadPart, WriterPart},
    error::{Error, Result},
    softap::{Event, SoftAp, SoftApConfig, DataReader},
    bytes_iter::BytesIter,
};

pub mod adapter;
pub mod error;
pub mod softap;

mod bytes_iter;
mod parser;
#[cfg(test)]
mod tests;

pub const ADAPTER_BUF_CAPACITY: usize = 512;
