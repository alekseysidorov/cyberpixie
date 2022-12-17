//! Cyberpixie service traits

use embedded_io::blocking::Seek;
use serde::{Deserialize, Serialize};

use crate::{
    proto::types::{DeviceInfo, Hertz, ImageId},
    ExactSizeRead,
};

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

/// Basic device services.
pub trait DeviceService {
    type Storage: DeviceStorage;
    /// Returns important device information necessary for handshake.
    fn device_info(&self) -> DeviceInfo;
    /// Returns handle to the device storage.
    fn storage(&self) -> Self::Storage;
}

/// A type definition to represent an image reader for a certain device.
pub type DeviceImage<'a, S> = Image<<S as DeviceStorage>::ImageRead<'a>>;

/// Device data storage.
pub trait DeviceStorage {
    type ImageRead<'a>: ExactSizeRead + Seek
    where
        Self: 'a;

    /// Returns a global configuration.
    fn config(&self) -> crate::Result<Config>;
    /// Sets a global configuration.
    fn set_config(&self, value: &Config) -> crate::Result<()>;
    /// Adds a new image.
    fn add_image<R: ExactSizeRead>(&self, refresh_rate: Hertz, image: R) -> crate::Result<ImageId>;
    /// Reads an image by ID.
    fn read_image(&self, id: ImageId) -> crate::Result<DeviceImage<'_, Self>>;
    /// Returns total images count.
    fn images_count(&self) -> crate::Result<u16>;
    /// Remove all stored images.
    fn clear_images(&self) -> crate::Result<()>;
}
