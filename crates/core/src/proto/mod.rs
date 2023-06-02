// // #![cfg_attr(not(test), no_std)]

// use embedded_io::blocking::Read;
// pub use nb;
// pub use nb_utils;
// pub use payload::PayloadReader;
// pub use postcard::Error as PayloadError;

// pub use crate::error::Error;

use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use self::types::{ImageId, ImageInfo, PeerInfo};

pub mod packet;
pub mod types;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
pub enum RequestHeader {
    Handshake(PeerInfo),
    AddImage(ImageInfo),
    /// Start showing image with the specified ID
    ShowImage(ImageId),
    /// Hide currently showing image
    HideImage,
    ClearImages,
    Debug,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
pub enum ResponseHeader {
    Empty,
    Handshake(PeerInfo),
    AddImage(ImageId),
    Error(crate::Error),
}

impl ResponseHeader {
    pub const fn empty(self) -> crate::Result<()> {
        match self {
            Self::Empty => Ok(()),
            Self::Error(err) => Err(err),
            _ => Err(crate::Error::UnexpectedResponse),
        }
    }

    pub const fn handshake(self) -> crate::Result<PeerInfo> {
        match self {
            Self::Handshake(info) => Ok(info),
            Self::Error(err) => Err(err),
            _ => Err(crate::Error::UnexpectedResponse),
        }
    }

    pub const fn add_image(self) -> crate::Result<ImageId> {
        match self {
            Self::AddImage(id) => Ok(id),
            Self::Error(err) => Err(err),
            _ => Err(crate::Error::UnexpectedResponse),
        }
    }
}

/// Possible header types.
#[derive(Serialize, PartialEq, Eq, Clone, Copy, Debug, MaxSize)]
#[serde(untagged)]
pub enum Headers {
    Request(RequestHeader),
    Response(ResponseHeader),
}

impl From<RequestHeader> for Headers {
    fn from(value: RequestHeader) -> Self {
        Self::Request(value)
    }
}

impl From<ResponseHeader> for Headers {
    fn from(value: ResponseHeader) -> Self {
        Self::Response(value)
    }
}
