#![cfg_attr(not(test), no_std)]

pub use crate::packet::{
    write_message_header, FirmwareInfo, IncomingMessage, PacketReader, MAX_HEADER_LEN,
};

pub mod types;

mod packet;
#[cfg(all(test, not(target_os = "none")))]
mod tests;
