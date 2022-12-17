//! Image reader wrapper over block reader.

use embedded_io::{
    blocking::{Read, Seek},
    Io, SeekFrom,
};

use super::{BlockReader, DEFAULT_BLOCK_SIZE};
use crate::ExactSizeRead;

#[derive(Debug)]
pub struct ImageReader<T, const N: usize = DEFAULT_BLOCK_SIZE>
where
    T: BlockReader<N>,
{
    block_reader: T,

    image_len: usize,
    bytes_read: usize,

    block: [u8; N],
}

impl<T, const N: usize> ImageReader<T, N>
where
    T: BlockReader<N>,
{
    pub fn new(block_reader: T, image_len: usize) -> Self {
        Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block: [0_u8; N],
        }
    }

    fn current_block(&self) -> usize {
        self.bytes_read / N
    }

    fn read_current_block_to_buf(&mut self) -> Result<(), T::Error> {
        let to = core::cmp::min(self.bytes_remaining(), N);
        let block = self.current_block();

        let buf = &mut self.block[0..to];
        self.block_reader.read_block(block, buf)?;
        Ok(())
    }
}

impl<T: BlockReader<N>, const N: usize> Io for ImageReader<T, N> {
    type Error = T::Error;
}

impl<T, const N: usize> ExactSizeRead for ImageReader<T, N>
where
    T: BlockReader<N>,
{
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<T: BlockReader<N>, const N: usize> Seek for ImageReader<T, N> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

impl<T, const N: usize> Read for ImageReader<T, N>
where
    T: BlockReader<N>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Just return if there is nothing to read or given buffer has zero size.
        if self.is_empty() || buf.is_empty() {
            return Ok(0);
        }
        // Fill block buffer if we are on the edge between blocks.
        if self.bytes_read % N == 0 {
            self.read_current_block_to_buf()?;
        }

        // Compute how much bytes we can copy to the outgoing buffer.
        let max_bytes_to_read = core::cmp::min(self.bytes_remaining(), buf.len());
        // Compute the position from where we should copy bytes from the block buffer.
        let from = self.bytes_read % N;
        // We can read the maximum number of bytes but no more than the length
        // of the remaining block part.
        let bytes_to_read = core::cmp::min(max_bytes_to_read, N - from);
        let to = from + bytes_to_read;
        // Perform copying bytes from block to outgoing buffer.
        buf[0..bytes_to_read].copy_from_slice(&self.block[from..to]);
        // Increment block read position
        self.bytes_read += bytes_to_read;
        Ok(bytes_to_read)
    }
}
