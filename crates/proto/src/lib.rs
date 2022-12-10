// #![cfg_attr(not(test), no_std)]

pub use nb;
pub use nb_utils;
pub use postcard::Error as PayloadError;

pub use crate::{
    error::Error,
    message::{Message, SimpleMessage},
    service::{Event as ServiceEvent, Service},
    transport::{Event as TransportEvent, PacketData, PacketKind, PacketWithPayload, Transport},
    types::{DeviceRole, FirmwareInfo, Handshake, Hertz},
};

pub mod error;
pub mod ng;

mod message;
mod service;
#[cfg(all(test, not(target_os = "none")))]
mod tests;
mod transport;
mod types;
