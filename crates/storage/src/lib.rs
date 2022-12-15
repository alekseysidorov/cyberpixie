#![cfg_attr(not(test), no_std)]

use core::usize;

use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use embedded_io::{blocking::Seek, Io};
use serde::{Deserialize, Serialize};

pub mod image_reader;

/// Block size used by default.
pub const BLOCK_SIZE: usize = 512;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    pub strip_len: u16,
    pub current_image: u16,
}

pub struct Image<R>
where
    R: ExactSizeRead + Seek,
{
    pub refresh_rate: Hertz,
    pub bytes: R,
}

/// Device data storage.
pub trait DeviceStorage {
    type Error;

    type ImageRead<'a>: ExactSizeRead + Seek
    where
        Self: 'a;

    /// Returns a global configuration.
    fn config(&self) -> Result<Config, Self::Error>;
    /// Sets a global configuration.
    fn set_config(&self, value: &Config) -> Result<(), Self::Error>;
    /// Adds a new image.
    fn add_image<R>(&self, refresh_rate: Hertz, image: R) -> Result<ImageId, Self::Error>
    where
        Self::Error: From<R::Error>,
        R: ExactSizeRead;
    /// Reads an image by ID.
    fn read_image(&self, id: ImageId) -> Result<Option<Image<Self::ImageRead<'_>>>, Self::Error>;
    // /// Remove all stored images.
    // fn clear_images(&self) -> Result<(), Self::Error>;
    //
    /// Returns total images count.
    fn images_count(&self) -> Result<u16, Self::Error>;
}

pub trait BlockReader<const BLOCK_SIZE: usize>: Io {
    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<(), Self::Error>;
}

impl<const BLOCK_SIZE: usize> BlockReader<BLOCK_SIZE> for &[u8] {
    fn read_block(&self, index: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let from = index * BLOCK_SIZE;
        let to = from + core::cmp::min(BLOCK_SIZE, buf.len());
        buf.copy_from_slice(&self[from..to]);
        Ok(())
    }
}
