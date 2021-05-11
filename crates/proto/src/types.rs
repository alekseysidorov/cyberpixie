use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FirmwareInfo {
    pub version: u32,
    pub strip_len: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct AddImage {
    pub refresh_rate: u32,
    pub strip_len: u16,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum MessageHeader {
    // Requests
    GetInfo,
    ClearImages,
    AddImage(AddImage),

    // Responses.
    Info(FirmwareInfo),
    Error(u16),
}
