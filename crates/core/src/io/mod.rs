//! An additional I/O traits and facilities.

pub mod image_reader;

/// The reader with the exact number of bytes to read.
pub trait ExactSizeRead {
    /// Return the total number of bytes, that should be read.
    // fn len(&self) -> usize;

    /// Returns the remaining bytes to read.
    fn bytes_remaining(&self) -> usize;
    /// Return true if there are remaining bytes to read.
    #[inline]
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
