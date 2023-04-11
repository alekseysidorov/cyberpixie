#![cfg_attr(not(any(feature = "std", test)), no_std)]

use embedded_io::blocking::Read;
pub use errors::{Error, Result};

pub mod errors;
pub mod proto;
pub mod service;
pub mod storage;

/// The Blocking reader with the exact number of bytes to read.
pub trait ExactSizeRead: Read {
    /// Return the total number of bytes, that should be read.
    // fn len(&self) -> usize;
    
    /// Returns the remaining bytes to read.
    fn bytes_remaining(&self) -> usize;
    /// Return true if there are remaining bytes to read.
    fn is_empty(&self) -> bool {
        self.bytes_remaining() == 0
    }
}

impl<T: ?Sized + ExactSizeRead> ExactSizeRead for &mut T {
    fn bytes_remaining(&self) -> usize {
        T::bytes_remaining(self)
    }
}

impl ExactSizeRead for &[u8] {
    fn bytes_remaining(&self) -> usize {
        self.len()
    }
}
