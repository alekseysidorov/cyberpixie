//! Image reader implementation

use std::fmt::Display;

use cyberpixie_core::{proto::types::ImageId, storage::BlockReader};
use esp_idf_sys::EspError;

use super::ImagesRegistry;

pub type ImageReader<'a> = cyberpixie_core::storage::ImageReader<BlockReaderImpl<'a>, Vec<u8>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BlockReadError(pub EspError);

impl Display for BlockReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BlockReadError {}

impl embedded_io::Error for BlockReadError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl From<EspError> for BlockReadError {
    fn from(value: EspError) -> Self {
        Self(value)
    }
}

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

impl<'a> embedded_io::Io for BlockReaderImpl<'a> {
    type Error = BlockReadError;
}

impl<'a, const N: usize> BlockReader<N> for BlockReaderImpl<'a> {
    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let idx = self.image_index.0;

        log::trace!("Filling block {block} [0..{}]", buf.len(),);
        self.registry
            .get_raw(&format!("img.{idx}.block.{block}"), buf)?;
        Ok(())
    }
}
