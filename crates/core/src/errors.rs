use core::fmt::Display;

use displaydoc::Display;
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

/// A specialized result type for Cyberpixie device.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur when processing messages.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Debug, Serialize, Deserialize, MaxSize,
)]
#[repr(u16)]
pub enum Error {
    /// The length of the strip does not match with the specified.
    StripLengthMismatch = 1,
    /// The length of the picture in bytes is not a multiple of "strip length" * "bytes per pixel".
    ImageLengthMismatch = 2,
    /// The transmitted message cannot be fitted into the device's memory.
    ImageTooBig = 3,
    /// This image repository on the device is full.
    ImageRepositoryFull = 4,
    /// The specified image index is greater than the total amount of the stored images.
    ImageNotFound = 5,
    /// Unexpected response to the request.
    UnexpectedResponse = 6,
    /// Unable to read bytes from storage.
    StorageRead = 7,
    /// Unable to write bytes to storage.
    StorageWrite = 8,
    /// Network read error.
    Network = 9,
    /// Data decoding error.
    Decode = 10,
    /// Data encoding error.
    Encode = 11,
    /// Unspecified or unknown error.
    Unspecified(u16),
}

impl Error {
    pub fn from_code(code: u16) -> Self {
        match code {
            1 => Self::StripLengthMismatch,
            2 => Self::ImageLengthMismatch,
            3 => Self::ImageTooBig,
            4 => Self::ImageRepositoryFull,
            5 => Self::ImageNotFound,
            6 => Self::UnexpectedResponse,
            7 => Self::StorageRead,
            8 => Self::StorageWrite,
            9 => Self::Network,
            10 => Self::Decode,
            11 => Self::Encode,

            other => Self::Unspecified(other),
        }
    }

    pub fn into_code(self) -> u16 {
        match self {
            Error::StripLengthMismatch => 1,
            Error::ImageLengthMismatch => 2,
            Error::ImageTooBig => 3,
            Error::ImageRepositoryFull => 4,
            Error::ImageNotFound => 5,
            Error::UnexpectedResponse => 6,
            Error::StorageRead => 7,
            Error::StorageWrite => 8,
            Error::Network => 9,
            Error::Decode => 10,
            Error::Encode => 11,

            Error::Unspecified(other) => other,
        }
    }

    /// Creates a new storage read error.
    pub fn storage_read<E>(_: E) -> Self
    where
        E: Display,
    {
        Self::StorageRead
    }

    pub fn storage_write<E>(_: E) -> Self
    where
        E: Display,
    {
        Self::StorageWrite
    }

    pub fn network<E>(_: E) -> Self
    where
        E: Display,
    {
        Self::Network
    }

    /// Creates a new decode data error.
    pub fn decode<E>(_: E) -> Self
    where
        E: Display,
    {
        Self::Decode
    }

    /// Creates a new encode data error.
    pub fn encode<E>(_: E) -> Self
    where
        E: Display,
    {
        Self::Encode
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "std")]
impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err)
    }
}
