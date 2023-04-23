// // #![cfg_attr(not(test), no_std)]

// use embedded_io::blocking::Read;
// pub use nb;
// pub use nb_utils;
// pub use payload::PayloadReader;
// pub use postcard::Error as PayloadError;

// pub use crate::error::Error;

use embedded_io::{blocking::Read, Io};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

use self::types::{ImageId, ImageInfo, PeerInfo};
use crate::ExactSizeRead;

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

#[derive(Debug, Clone, Copy)]
pub struct PayloadReader<T> {
    payload_len: usize,
    bytes_remaining: usize,
    inner: T,
}

impl<T: Read> PayloadReader<T> {
    pub const fn new(inner: T, payload_len: usize) -> Self {
        Self {
            payload_len,
            bytes_remaining: payload_len,
            inner,
        }
    }

    pub const fn len(&self) -> usize {
        self.payload_len
    }

    pub const fn is_empty(&self) -> bool {
        self.payload_len == 0
    }
}

impl<T: Read> Io for PayloadReader<T> {
    type Error = T::Error;
}

impl<T: Read> Read for PayloadReader<T> {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Don't read more bytes the from buffer than necessary.
        if buf.len() > self.bytes_remaining {
            buf = &mut buf[0..self.bytes_remaining];
        }

        let bytes_read = self.inner.read(buf)?;
        self.bytes_remaining -= bytes_read;
        Ok(bytes_read)
    }
}

impl<T: Read> ExactSizeRead for PayloadReader<T> {
    fn bytes_remaining(&self) -> usize {
        self.bytes_remaining
    }
}

impl<'a> From<&'a [u8]> for PayloadReader<&'a [u8]> {
    fn from(inner: &'a [u8]) -> Self {
        Self::new(inner, inner.len())
    }
}

impl<'a> From<&'a str> for PayloadReader<&'a [u8]> {
    fn from(inner: &'a str) -> Self {
        Self::new(inner.as_bytes(), inner.len())
    }
}
