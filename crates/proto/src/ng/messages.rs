use endian_codec::PackedSize;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use super::{types::{Handshake, ImageId, ImageInfo}, transport::Packet};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
pub enum MessageHeader {
    RequestHandshake(Handshake),
    RequestAddImage(ImageInfo),
    Debug,

    ResponseHandshake(Handshake),
    ResponseAddImage(ImageId),
}

impl MessageHeader {
    pub const MAX_LEN: usize = Self::POSTCARD_MAX_SIZE + Packet::PACKED_LEN;
}
