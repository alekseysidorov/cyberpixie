use core::str::FromStr;

use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DeviceRole {
    /// A control device such as a telephone or laptop.
    Host = 0,
    /// A main device that receives commands directly from the host and then re-sends them
    /// to the slave devices if they exist.
    Main = 1,
    /// A secondary device that executes commands from the main one.
    Secondary = 2,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FirmwareInfo {
    pub version: [u8; 4],
    pub device_id: [u32; 4],
    pub role: DeviceRole,
    pub strip_len: u16,
    pub images_count: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct AddImage {
    pub refresh_rate: Hertz,
    pub strip_len: u16,
    pub bytes_len: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct Handshake {
    pub device_id: [u32; 4],
    pub role: DeviceRole,
    pub group_id: Option<u32>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MessageHeader {
    // Requests.
    HandshakeRequest(Handshake),
    GetInfo,
    ClearImages,
    AddImage(AddImage),
    ShowImage(u16),

    // Responses.
    Ok,
    HandshakeResponse(Handshake),
    ImageAdded(u16),
    Info(FirmwareInfo),
    Error(u16),
}

impl MessageHeader {
    pub const MAX_LEN: usize = 128;
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Hash)]
pub struct Hertz(pub u32);

impl FromStr for Hertz {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str(s).map(Self)
    }
}

impl From<u32> for Hertz {
    fn from(inner: u32) -> Self {
        Self(inner)
    }
}
