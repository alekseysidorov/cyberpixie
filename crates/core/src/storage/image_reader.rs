//! Image reader wrapper over block reader.

use embedded_io::{
    blocking::{Read, Seek},
    Io, SeekFrom,
};

use super::{BlockReader, DEFAULT_BLOCK_SIZE};
use crate::ExactSizeRead;

#[derive(Debug)]
pub struct ImageReader<T, B, const N: usize = DEFAULT_BLOCK_SIZE>
where
    T: BlockReader<N>,
{
    block_reader: T,

    image_len: usize,
    bytes_read: usize,

    block: B,
}

impl<T, const N: usize> ImageReader<T, [u8; N], N>
where
    T: BlockReader<N>,
{
    pub fn new_in_array(block_reader: T, image_len: usize) -> Self {
        Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block: [0_u8; N],
        }
    }
}

impl<T, B, const N: usize> ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    pub fn new(block_reader: T, image_len: usize, mut block: B) -> Self {
        assert!(
            block.as_mut().len() >= N,
            "Given buffer has not enough capacity to store the entire block content"
        );

        Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block,
        }
    }

    fn current_block(&self) -> usize {
        self.bytes_read / N
    }

    fn read_current_block_to_buf(&mut self) -> Result<(), T::Error> {
        let to = core::cmp::min(self.bytes_remaining(), N);
        let block = self.current_block();

        let buf = &mut self.block.as_mut()[0..to];
        self.block_reader.read_block(block, buf)?;
        Ok(())
    }
}

impl<T: BlockReader<N>, B, const N: usize> Io for ImageReader<T, B, N> {
    type Error = T::Error;
}

impl<T, B, const N: usize> ExactSizeRead for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<T, B, const N: usize> Seek for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    fn seek(&mut self, seek: SeekFrom) -> Result<u64, Self::Error> {
        // Compute a new image read position
        self.bytes_read = match seek {
            SeekFrom::Start(pos) => pos as usize,
            // In this project, we only have to read an image from the beginning,
            // so we don't need to implement the whole seek functionality
            SeekFrom::Current(_pos) => unimplemented!(),
            SeekFrom::End(_pos) => unimplemented!(),
        };
        // Reread the block corresponding to the new read position
        self.read_current_block_to_buf()?;
        Ok(self.bytes_read as u64)
    }
}

impl<T, B, const N: usize> Read for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
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
        buf[0..bytes_to_read].copy_from_slice(&self.block.as_mut()[from..to]);
        // Increment block read position
        self.bytes_read += bytes_to_read;
        Ok(bytes_to_read)
    }
}
