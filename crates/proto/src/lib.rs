#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use crate::{
    packet::{Error, FirmwareInfo, Message, PacketReader, SimpleMessage, MAX_HEADER_LEN},
    service::{Service, ServiceEvent},
};
pub use postcard::Error as PayloadError;

pub mod transport;
pub mod types;
pub mod service_ng;

mod packet;
mod service;
#[cfg(all(test, not(target_os = "none")))]
mod tests;
