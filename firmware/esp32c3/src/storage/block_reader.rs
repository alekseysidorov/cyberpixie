//! Image reader implementation

use cyberpixie_proto::{types::ImageId, ExactSizeRead};
use embedded_io::{
    blocking::{Read, Seek},
    Io, SeekFrom,
};
use esp_idf_sys::EspError;

use super::{ImagesRegistry, BLOCK_SIZE};

#[derive(Debug)]
pub struct BlockReader<'a> {
    registry: &'a ImagesRegistry,
    image_index: ImageId,
    image_len: usize,

    bytes_read: usize,
    block: [u8; BLOCK_SIZE],
}

impl<'a> BlockReader<'a> {
    pub fn new(
        registry: &'a ImagesRegistry,
        image_index: ImageId,
        image_len: u32,
    ) -> Result<Self, EspError> {
        let image_len = image_len as usize;

        let mut reader = Self {
            registry,
            image_index,
            image_len,
            bytes_read: 0,
            block: [0_u8; BLOCK_SIZE],
        };
        reader.fill_block()?;
        Ok(reader)
    }

    fn fill_block(&mut self) -> Result<(), EspError> {
        let to = std::cmp::min(self.bytes_remaining(), BLOCK_SIZE);
        let idx = self.image_index.0;
        let block = self.current_block();

        log::info!(
            "Filling block {block} [0..{to}], bytes_remaining: {}",
            self.bytes_remaining()
        );

        let buf = &mut self.block[0..to];
        self.registry
            .get_raw(&format!("img.{idx}.block.{block}"), buf)?;
        Ok(())
    }

    fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    fn current_block(&self) -> usize {
        self.bytes_read / BLOCK_SIZE
    }
}

impl<'a> Io for BlockReader<'a> {
    type Error = std::io::Error;
}

impl<'a> ExactSizeRead for BlockReader<'a> {
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<'a> Seek for BlockReader<'a> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

impl<'a> Read for BlockReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}
