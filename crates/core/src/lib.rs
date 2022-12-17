#![cfg_attr(not(any(feature = "std", test)), no_std)]

use core::{fmt::{Debug, Display}, usize};

use cyberpixie_proto::{
    types::{DeviceInfo, Hertz, ImageId},
    ExactSizeRead,
};
use embedded_io::blocking::Seek;
use serde::{Deserialize, Serialize};

pub mod image_reader;

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

#[derive(Debug)]
pub enum AddImageError<R, P> {
    PayloadReadError(R),
    ImageWriteError(P),
}

impl<R, P> embedded_io::Error for AddImageError<R, P>
where
    R: Debug,
    P: Debug,
{
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}


impl<R, P> Display for AddImageError<R, P>
where
    R: Display,
    P: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AddImageError::PayloadReadError(err) => write!(f, "Read payload error: {err}"),
            AddImageError::ImageWriteError(err) => write!(f, "Write image error: {err}"),
        }
    }
}

#[cfg(feature = "std")]
impl<R, P> std::error::Error for AddImageError<R, P>
where
    R: Debug + Display,
    P: Debug + Display,
{
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
    fn add_image<R>(
        &self,
        refresh_rate: Hertz,
        image: R,
    ) -> Result<ImageId, AddImageError<R::Error, Self::Error>>
    where
        R: ExactSizeRead;
    /// Reads an image by ID.
    fn read_image(&self, id: ImageId) -> Result<Option<Image<Self::ImageRead<'_>>>, Self::Error>;
    /// Returns total images count.
    fn images_count(&self) -> Result<u16, Self::Error>;
    /// Remove all stored images.
    fn clear_images(&self) -> Result<(), Self::Error>;
}

/// Basic device services.
pub trait DeviceService {
    type Storage: DeviceStorage;
    /// Returns important device information necessary for handshake.
    fn device_info(&self) -> DeviceInfo;
    /// Returns handle to the device storage.
    fn storage(&self) -> Self::Storage;
}
