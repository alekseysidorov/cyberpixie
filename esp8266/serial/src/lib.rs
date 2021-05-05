#![cfg_attr(not(test), no_std)]

pub use crate::error::{Result, Error};

pub mod adapter;
pub mod softap;
pub mod error;

#[cfg(test)]
mod tests;
mod parser;

pub const ADAPTER_BUF_CAPACITY: usize = 512;
