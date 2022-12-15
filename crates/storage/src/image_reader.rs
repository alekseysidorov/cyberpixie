//! Image reader wrapper over block reader.

use core::convert::Infallible;

use cyberpixie_proto::ExactSizeRead;
use embedded_io::{Io, blocking::{Read, Seek}, SeekFrom};

use crate::{BlockReader, BLOCK_SIZE};

#[derive(Debug)]
pub struct ImageReader<T, const N: usize = BLOCK_SIZE>
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
    pub fn new(block_reader: T, image_len: usize) -> Result<Self, T::Error> {
        let mut image_reader = Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block: [0_u8; N],
        };
        image_reader.fill_block()?;
        Ok((image_reader))
    }

    fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    fn current_block(&self) -> usize {
        self.bytes_read / BLOCK_SIZE
    }

    fn fill_block(&mut self) -> Result<(), T::Error> {
        let to = core::cmp::min(self.bytes_remaining(), BLOCK_SIZE);
        let block = self.current_block();

        log::info!(
            "Filling block {block} [0..{to}], bytes_remaining: {}",
            self.bytes_remaining()
        );

        let buf = &mut self.block[0..to];
        self.block_reader.read_block(block, buf)?;
        Ok(())
    }
}

impl<T: BlockReader<N>, const N: usize> Io for ImageReader<T, N> {
    type Error = Infallible;
}

impl<T, const N: usize> ExactSizeRead for ImageReader<T, N> where T: BlockReader<N> {
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<T: BlockReader<N>, const N: usize> Seek for ImageReader<T, N> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

impl<T, const N: usize> Read for ImageReader<T, N> where T: BlockReader<N> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}
