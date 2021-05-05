#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::Adapter,
    error::{Error, Result},
    softap::{Event, SoftAp, SoftApConfig},
};

pub mod adapter;
pub mod error;
pub mod softap;

mod parser;
#[cfg(test)]
mod tests;

pub const ADAPTER_BUF_CAPACITY: usize = 512;
