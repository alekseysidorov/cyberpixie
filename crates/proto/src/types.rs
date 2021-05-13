use core::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FirmwareInfo {
    pub version: u32,
    pub strip_len: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct AddImage {
    pub refresh_rate: Hertz,
    pub strip_len: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MessageHeader {
    // Requests.
    GetInfo,
    ClearImages,
    AddImage(AddImage),

    // Responses.
    Ok,
    ImageAdded(u16),
    Info(FirmwareInfo),
    Error(u16),
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
