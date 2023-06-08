#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self, clippy::missing_const_for_fn)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

pub use errors::{Error, Result};

pub mod errors;
pub mod proto;
pub mod io;

/// The maximum effective length of the pixel strip.
///
/// It doesn't make sense to create pixel devices with strip longer than this one,
/// the ws2812 protocol has not enough refresh rate.
pub const MAX_STRIP_LEN: usize = 48;

/// The reader with the exact number of bytes to read.
pub trait ExactSizeRead {
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
    #[inline]
    fn bytes_remaining(&self) -> usize {
        T::bytes_remaining(self)
    }
}

impl ExactSizeRead for &[u8] {
    #[inline]
    fn bytes_remaining(&self) -> usize {
        self.len()
    }
}
