//! Storage traits and facilities.

use embedded_io::Io;

pub use self::image_reader::ImageReader;

mod image_reader;

/// Block size used by default.
pub const DEFAULT_BLOCK_SIZE: usize = 512;

/// Auxiliary trait describing reading from block devices.
pub trait BlockReader<const BLOCK_SIZE: usize>: Io {
    /// Read block content into the specified buffer.
    fn read_block(&self, index: usize, buf: &mut [u8]) -> Result<(), Self::Error>;
}

impl<const BLOCK_SIZE: usize> BlockReader<BLOCK_SIZE> for &[u8] {
    fn read_block(&self, index: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let from = index * BLOCK_SIZE;
        let to = from + core::cmp::min(BLOCK_SIZE, buf.len());
        buf.copy_from_slice(&self[from..to]);
        Ok(())
    }
}
