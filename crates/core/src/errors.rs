use core::fmt::Debug;

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
    ImageRepositoryIsFull = 4,
    /// The images repository is empty.
    ImageRepositoryIsEmpty = 5,
    /// The specified image index is greater than the total amount of the stored images.
    ImageNotFound = 6,
    /// Unexpected response to the request.
    UnexpectedResponse = 7,
    /// Unable to read bytes from storage.
    StorageRead = 8,
    /// Unable to write bytes to storage.
    StorageWrite = 9,
    /// Network read error.
    Network = 10,
    /// Data decoding error.
    Decode = 11,
    /// Data encoding error.
    Encode = 12,
    /// Image render is busy by the another task.
    ImageRenderIsBusy = 13,
    /// Internal device error.
    Internal = 14,
    /// Unspecified or unknown error.
    Unspecified(u16),
}

impl Error {
    #[must_use]
    pub const fn from_code(code: u16) -> Self {
        match code {
            1 => Self::StripLengthMismatch,
            2 => Self::ImageLengthMismatch,
            3 => Self::ImageTooBig,
            4 => Self::ImageRepositoryIsFull,
            5 => Self::ImageRepositoryIsEmpty,
            6 => Self::ImageNotFound,
            7 => Self::UnexpectedResponse,
            8 => Self::StorageRead,
            9 => Self::StorageWrite,
            10 => Self::Network,
            11 => Self::Decode,
            12 => Self::Encode,
            13 => Self::ImageRenderIsBusy,
            42 => Self::Internal,

            other => Self::Unspecified(other),
        }
    }

    #[must_use]
    pub const fn into_code(self) -> u16 {
        match self {
            Self::StripLengthMismatch => 1,
            Self::ImageLengthMismatch => 2,
            Self::ImageTooBig => 3,
            Self::ImageRepositoryIsFull => 4,
            Self::ImageRepositoryIsEmpty => 5,
            Self::ImageNotFound => 6,
            Self::UnexpectedResponse => 7,
            Self::StorageRead => 8,
            Self::StorageWrite => 9,
            Self::Network => 10,
            Self::Decode => 11,
            Self::Encode => 12,
            Self::ImageRenderIsBusy => 13,
            Self::Internal => 14,

            Self::Unspecified(other) => other,
        }
    }

    /// Creates a new storage read error.
    pub fn storage_read<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("A storage read error occurred: {err:?}");
        Self::StorageRead
    }

    pub fn storage_write<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("A storage write error occurred: {err:?}");
        Self::StorageWrite
    }

    pub fn network<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("A network error occurred: {err:?}");
        Self::Network
    }

    /// Creates a new decode data error.
    #[must_use]
    pub fn decode<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("A decoding error occurred: {err:?}");
        Self::Decode
    }

    /// Creates a new encode data error.
    #[must_use]
    pub fn encode<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("An encoding error occurred: {err:?}");
        Self::Encode
    }

    /// Creates a new internal error.
    pub fn internal<E>(err: E) -> Self
    where
        E: Debug,
    {
        log::warn!("A network error occurred: {err:?}");
        Self::Internal
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[cfg(feature = "std")]
impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        Self::new(std::io::ErrorKind::Other, err)
    }
}
