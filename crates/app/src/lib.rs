//! Cyberpixie application

#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![feature(async_fn_in_trait)]
// Linter configuration

pub use cyberpixie_core as core;
use cyberpixie_core::{
    proto::types::{DeviceInfo, FirmwareInfo, Hertz, ImageId},
    storage::Image,
    ExactSizeRead,
};
pub use cyberpixie_core::{Error as CyberpixieError, Result as CyberpixieResult};
pub use cyberpixie_network as network;
use embedded_io::blocking::{Read, Seek};
use embedded_nal::TcpFullStack;
use network::PayloadReader;
use serde::{Deserialize, Serialize};

pub use crate::app::{App, Connections};

mod app;
pub mod asynch;

/// Default application network port.
pub const NETWORK_PORT: u16 = 1800;

/// Board-specific components
///
/// Including the network stack, image and other data storage and LED strip rendering task.
pub trait Board {
    /// Type provides the internal storage functionality.
    type Storage: Storage;
    /// Type provides the network stack.
    type NetworkStack: TcpFullStack;
    /// Type provides a LED strip pictures rendering task.
    type RenderTask;
    /// Returns all board components.
    ///
    /// This method brings the component ownership to the caller and can be invoked only once.
    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)>;
    /// Starts LED strip rendering task.
    ///
    /// To prevent data races, this method takes [`Self::Storage`] the entirely, making
    /// it impossible to modify it while the image rendering task is being executed.
    fn start_rendering(
        &mut self,
        storage: Self::Storage,
        image_id: ImageId,
    ) -> CyberpixieResult<Self::RenderTask>;
    /// Stops a LED strip rendering task and returns back previously borrowed storage.
    fn stop_rendering(&mut self, handle: Self::RenderTask) -> CyberpixieResult<Self::Storage>;
    /// Returns a board firmware information.
    fn firmware_info(&self) -> FirmwareInfo;

    /// Shows a debug message.
    ///
    /// Default implementation just do nothing.
    fn show_debug_message<R: Read>(&self, mut payload: PayloadReader<R>) -> CyberpixieResult<()> {
        while payload.bytes_remaining() != 0 {
            let mut byte = [0_u8];
            payload.read(&mut byte).map_err(CyberpixieError::network)?;
        }
        Ok(())
    }
}

/// A global application configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Configuration {
    /// The number of LEDs in the strip.
    pub strip_len: u16,
    /// Index of the picture which will be show by default.
    pub current_image: Option<ImageId>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            strip_len: 24,
            current_image: None,
        }
    }
}

/// A type definition to represent an image reader for a certain device.
pub type ImageReader<'a, S> = Image<<S as Storage>::ImageRead<'a>>;

/// Board internal storage.
pub trait Storage: Send + 'static {
    /// Image reader type.
    type ImageRead<'a>: Read + ExactSizeRead + Seek
    where
        Self: 'a;
    /// Returns an application configuration.
    fn config(&mut self) -> CyberpixieResult<Configuration>;
    /// Updates an application configuration.
    ///
    /// # Notice for the board developers
    ///
    /// - You should check the current image index for the boundaries
    /// - You must invoke [`Self::clear_images`] method if the strip length changes.
    fn set_config(&mut self, config: Configuration) -> CyberpixieResult<()>;
    /// Adds a new image.
    fn add_image<R: Read + ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        image: R,
    ) -> CyberpixieResult<ImageId>;
    /// Reads an image with the given identifier.
    fn read_image(&mut self, id: ImageId) -> CyberpixieResult<ImageReader<'_, Self>>;
    /// Returns total saved images count.
    fn images_count(&mut self) -> CyberpixieResult<ImageId>;
    /// Remove all stored images.
    ///
    /// # Notice for the board developers
    ///
    /// - You should set the current image ID to the `None` in the board configuration.
    fn clear_images(&mut self) -> CyberpixieResult<()>;

    /// Adds a new image.
    async fn add_image_async<R: embedded_io::asynch::Read + ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        image: R,
    ) -> CyberpixieResult<ImageId>;

    /// Sets an index of image that will be shown.
    fn set_current_image_id<I>(&mut self, id: I) -> CyberpixieResult<()>
    where
        I: Into<Option<ImageId>>,
    {
        let id = id.into();
        // Check preconditions.
        if id >= Some(self.images_count()?) {
            return Err(CyberpixieError::ImageNotFound);
        }

        let mut config = self.config()?;
        config.current_image = id;
        self.set_config(config)
    }

    /// Returns an index of image that will be shown.
    fn current_image_id(&mut self) -> CyberpixieResult<Option<ImageId>> {
        Ok(self.config()?.current_image)
    }

    /// Switches to a next image, if it reaches the last image it turns back to the first image.
    fn switch_to_next_image(&mut self) -> CyberpixieResult<Option<ImageId>> {
        let Some(mut current_image) = self.current_image_id()? else {
                return Ok(None)
            };

        current_image.0 += 1;
        if current_image == self.images_count()? {
            current_image.0 = 0;
        }
        self.set_current_image_id(current_image)?;
        Ok(Some(current_image))
    }

    /// Reads a current image.
    fn current_image(&mut self) -> CyberpixieResult<ImageReader<'_, Self>> {
        let image_id = self
            .current_image_id()?
            .ok_or(CyberpixieError::ImageRepositoryIsEmpty)?;
        self.read_image(image_id)
    }
}

impl<T: Storage> Storage for &'static mut T {
    type ImageRead<'a> = T::ImageRead<'a>;

    fn config(&mut self) -> CyberpixieResult<Configuration> {
        T::config(self)
    }

    fn set_config(&mut self, config: Configuration) -> CyberpixieResult<()> {
        T::set_config(self, config)
    }

    fn add_image<R: Read + ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        image: R,
    ) -> CyberpixieResult<ImageId> {
        T::add_image(self, refresh_rate, image)
    }

    fn read_image(&mut self, id: ImageId) -> CyberpixieResult<ImageReader<'_, Self>> {
        T::read_image(self, id)
    }

    fn images_count(&mut self) -> CyberpixieResult<ImageId> {
        T::images_count(self)
    }

    fn clear_images(&mut self) -> CyberpixieResult<()> {
        T::clear_images(self)
    }

    async fn add_image_async<R: embedded_io::asynch::Read + ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        image: R,
    ) -> CyberpixieResult<ImageId> {
        T::add_image_async(self, refresh_rate, image).await
    }
}

pub(crate) fn read_device_info<S: Storage>(storage: &mut S) -> CyberpixieResult<DeviceInfo> {
    let config = storage.config()?;
    Ok(DeviceInfo {
        strip_len: config.strip_len,
        images_count: storage.images_count()?,
        current_image: config.current_image,
        active: false,
    })
}
