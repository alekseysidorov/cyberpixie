//! Image reader implementation

use cyberpixie_proto::{types::ImageId, ExactSizeRead};
use cyberpixie_storage::BlockReader;
use embedded_io::{
    blocking::{Read, Seek},
    Io, SeekFrom,
};
use esp_idf_sys::EspError;

use super::{ImagesRegistry, BLOCK_SIZE};

#[derive(Debug)]
pub struct BlockReaderImpl<'a> {
    registry: &'a ImagesRegistry,
    image_index: ImageId,
}

impl<'a> BlockReaderImpl<'a> {
    pub fn new(registry: &'a ImagesRegistry, image_index: ImageId) -> Self {
        Self {
            registry,
            image_index,
        }
    }
}

impl<'a> BlockReader<BLOCK_SIZE> for BlockReaderImpl<'a> {
    type Error = EspError;

    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let idx = self.image_index.0;

        log::info!("Filling block {block} [0..{}]", buf.len(),);
        self.registry
            .get_raw(&format!("img.{idx}.block.{block}"), buf)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct GenericImageReader<T: BlockReader<BLOCK_SIZE>> {
    block_reader: T,

    image_len: usize,
    bytes_read: usize,

    block: [u8; BLOCK_SIZE],
}

pub type ImageReader<'a> = GenericImageReader<BlockReaderImpl<'a>>;

impl<'a> ImageReader<'a> {
    pub fn new(
        registry: &'a ImagesRegistry,
        image_index: ImageId,
        image_len: u32,
    ) -> Result<Self, EspError> {
        let image_len = image_len as usize;

        let mut reader = Self {
            block_reader: BlockReaderImpl::new(registry, image_index),
            image_len,
            bytes_read: 0,
            block: [0_u8; BLOCK_SIZE],
        };
        reader.fill_block()?;
        Ok(reader)
    }

    fn fill_block(&mut self) -> Result<(), EspError> {
        let to = std::cmp::min(self.bytes_remaining(), BLOCK_SIZE);
        let block = self.current_block();

        log::info!(
            "Filling block {block} [0..{to}], bytes_remaining: {}",
            self.bytes_remaining()
        );

        let buf = &mut self.block[0..to];
        self.block_reader.read_block(block, buf)?;
        Ok(())
    }

    fn bytes_read(&self) -> usize {
        self.bytes_read
    }

    fn current_block(&self) -> usize {
        self.bytes_read / BLOCK_SIZE
    }
}

impl<'a> Io for ImageReader<'a> {
    type Error = std::io::Error;
}

impl<'a> ExactSizeRead for ImageReader<'a> {
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<'a> Seek for ImageReader<'a> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

impl<'a> Read for ImageReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        todo!()
    }
}
