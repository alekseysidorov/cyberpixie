#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use crate::{
    error::Error,
    message::{Message, SimpleMessage},
    service::Service,
    transport::{Packet, PacketData, PacketKind, Transport, PacketWithPayload},
    types::{DeviceRole, FirmwareInfo, Hertz},
};
pub use postcard::Error as PayloadError;

pub mod error;

mod message;
mod service;
#[cfg(all(test, not(target_os = "none")))]
mod tests;
mod transport;
mod types;
