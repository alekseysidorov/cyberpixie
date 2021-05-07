use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FirmwareInfo {
    pub version: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct AddImage {
    pub refresh_rate: u32,
    pub len: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Request {
    GetInfo,
    AddImage(AddImage),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Response {
    Info(FirmwareInfo),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
    Error(u16),
}
