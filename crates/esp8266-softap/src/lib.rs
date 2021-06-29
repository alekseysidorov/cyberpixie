#![cfg_attr(not(test), no_std)]

pub use crate::{
    adapter::{Adapter, ReadPart, WritePart},
    error::{Error, Result},
    softap::SoftApConfig,
    tcp_socket::{Data, Event, TcpSocket},
};
pub use no_std_net as net;

pub mod adapter;
pub mod error;
pub mod softap;

mod parser;
mod tcp_socket;
#[cfg(test)]
mod tests;

pub const ADAPTER_BUF_CAPACITY: usize = 640;
