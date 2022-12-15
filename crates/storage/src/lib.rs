#![cfg_attr(not(test), no_std)]

use core::{convert::Infallible, usize};

use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use embedded_io::blocking::Seek;
use serde::{Deserialize, Serialize};

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

pub trait BlockReader<const B: usize> {
    type Error;

    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<(), Self::Error>;
}

impl<const B: usize, const N: usize> BlockReader<B> for [u8; N] {
    type Error = Infallible;

    fn read_block(&self, index: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let from = index * N;
        let to = from + N;
        buf.copy_from_slice(&self[from..to]);
        Ok(())
    }
}
