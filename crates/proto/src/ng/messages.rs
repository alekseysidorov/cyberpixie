use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use super::types::{Handshake, ImageId, ImageInfo};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
pub enum MessageHeader {
    RequestHandshake(Handshake),
    RequestAddImage(ImageInfo),

    ResponseHandshake(Handshake),
    ResponseAddImage(ImageId),
}

impl MessageHeader {
    pub const MAX_SIZE: usize = Self::POSTCARD_MAX_SIZE;
}
