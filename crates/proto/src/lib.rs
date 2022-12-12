// #![cfg_attr(not(test), no_std)]

pub use nb;
pub use nb_utils;
pub use postcard::Error as PayloadError;

pub use crate::error::Error;
pub use headers::MessageHeader;
pub use payload::PayloadReader;

pub mod error;
pub mod packet;
pub mod types;

mod headers;
mod payload;

/// The Blocking reader with the exact number of bytes to read.
pub trait ExactSizedRead {
    /// Return the total number of bytes, that should be read.
    fn bytes_len(&self) -> usize;
    /// Returns the remaining bytes to read.
    fn bytes_remaining(&self) -> usize;
    /// Return true if there are remaining bytes to read.
    fn is_empty(&self) -> bool {
        self.bytes_len() == 0
    }
}
