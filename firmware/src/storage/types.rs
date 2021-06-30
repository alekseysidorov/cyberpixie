use endian_codec::{DecodeLE, EncodeLE, PackedSize};

#[derive(Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct Header {
    pub version: u8,
    pub images_count: u16,
    pub vacant_block: u16,
}

#[derive(Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct ImageDescriptor {
    pub block_number: u16,
    pub image_len: u32,
    pub refresh_rate: u32,
}

impl Header {
    pub const VERSION: u8 = 1;
}
