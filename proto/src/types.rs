use serde::{Deserialize, Serialize};

use crate::Packet;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct AddImage {
    pub refresh_rate: u32,
    pub len: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Request {
    AddImage(AddImage),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Response {
    Ok,
    Error(u16),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Message<'a> {
    Request(Request),
    Response(Response),
    Bytes(&'a [u8]),
}

impl<'a> Message<'a> {
    pub fn from_packet(packet: &'a Packet) -> postcard::Result<Self> {
        postcard::from_bytes(packet.as_ref())
    }

    pub fn to_packet(self) -> postcard::Result<Packet> {
        let mut packet = Packet::empty();

        let bytes_used = postcard::to_slice(&self, packet.buf.as_mut())?;
        packet.size = bytes_used.len() as u16;
        Ok(packet)
    }
}
