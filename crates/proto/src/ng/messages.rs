use endian_codec::PackedSize;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use super::{types::{DeviceInfo, ImageId, ImageInfo}, transport::Packet};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
pub enum MessageHeader {
    RequestHandshake(DeviceInfo),
    RequestAddImage(ImageInfo),
    Debug,

    ResponseHandshake(DeviceInfo),
    ResponseAddImage(ImageId),
}

impl MessageHeader {
    pub const MAX_LEN: usize = Self::POSTCARD_MAX_SIZE + Packet::PACKED_LEN;
}
