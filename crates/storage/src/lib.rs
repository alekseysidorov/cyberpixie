#![cfg_attr(not(test), no_std)]

use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use embedded_io::blocking::{Read, Seek};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    pub strip_len: u16,
    pub current_image: u16,
}

pub struct ImageReader<R>
where
    R: ExactSizeRead + Seek,
{
    pub refresh_rate: Hertz,
    inner: R,
}

/// Device data storage.
pub trait DeviceStorage {
    type Error;

    // type ImageRead<'a>: Read + Seek + ExactSizedRead
    // where
    //     Self: 'a;

    /// Returns a global configuration.
    fn config(&self) -> Result<Config, Self::Error>;
    /// Sets a global configuration.
    fn set_config(&self, value: &Config) -> Result<(), Self::Error>;
    // TODO

    /// Adds a new image.
    fn add_image<R>(&self, refresh_rate: Hertz, image: R) -> Result<ImageId, Self::Error>
    where
        Self::Error: From<R::Error>,
        R: ExactSizeRead;
    // /// Reads an image by ID.
    // fn read_image(&self, id: ImageId) -> Option<ImageReader<Self::ImageRead<'_>>>;
    // /// Remove all stored images.
    // fn clear_images(&self) -> Result<(), Self::Error>;
    //
    /// Returns total images count.
    fn images_count(&self) -> Result<u16, Self::Error>;
}
