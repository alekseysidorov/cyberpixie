#![cfg_attr(not(test), no_std)]

use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizedRead,
};
use embedded_io::blocking::{Read, Seek};
use serde::{Deserialize, Serialize};

pub struct Config {
    pub strip_len: u16,
    pub current_image: u16,
}

pub struct ImageReader<R>
where
    R: Read + ExactSizedRead + Seek,
{
    pub refresh_rate: Hertz,
    inner: R,
}

/// Device data storage.
pub trait DeviceStorage: embedded_io::Io {
    type ImageRead<'a>: Read + Seek + ExactSizedRead
    where
        Self: 'a;

    /// Returns a global configuration.
    fn config(&self) -> Result<Config, Self::Error>;
    /// Sets a global configuration.
    fn set_config(&self, value: &Config) -> Result<Config, Self::Error>;
    /// Adds a new image.
    fn add_image<R: Read>(&self, refresh_rate: Hertz, bytes: R) -> Result<ImageId, Self::Error>
    where
        Self::Error: From<R::Error>;
    /// Reads an image by ID.
    fn read_image(&self, id: ImageId) -> Option<ImageReader<Self::ImageRead<'_>>>;
    /// Remove all stored images.
    fn clear_images(&self) -> Result<(), Self::Error>;
    /// Returns total images count.
    fn images_count(&self) -> u16;
}
