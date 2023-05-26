//! Cyberpixie application

use cyberpixie_core::{
    proto::types::{FirmwareInfo, Hertz, ImageId},
    service::Image,
    ExactSizeRead,
};
pub use cyberpixie_core::{Error as CyberpixieError, Result as CyberpixieResult};
use embedded_io::blocking::Seek;
use embedded_nal::TcpFullStack;
use serde::{Deserialize, Serialize};

pub use crate::app::App;

mod app;

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
}

/// A global application configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Configuration {
    /// The number of LEDs in the strip.
    pub strip_len: u16,
    /// Index of the picture which will be show by default.
    pub current_image: Option<ImageId>,
}

/// A type definition to represent an image reader for a certain device.
pub type ImageReader<'a, S> = Image<<S as Storage>::ImageReader<'a>>;

/// Board internal storage.
pub trait Storage: Send + 'static {
    /// Image reader type.
    type ImageReader<'a>: ExactSizeRead + Seek
    where
        Self: 'a;
    /// Returns an application configuration.
    fn config(&self) -> CyberpixieResult<Configuration>;
    /// Updates an application configuration.
    ///
    /// # Notice for the board developers
    ///
    /// - You should check the current image index for the boundaries
    /// - You must invoke [`Self::clear_images`] method if the strip length changes.
    fn set_config(&mut self, config: Configuration) -> CyberpixieResult<()>;
    /// Adds a new image.
    fn add_image<R: ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        image: R,
    ) -> CyberpixieResult<ImageId>;
    /// Reads an image with the given identifier.
    fn read_image(&self, id: ImageId) -> CyberpixieResult<ImageReader<'_, Self>>;
    /// Returns total saven images count.
    fn images_count(&self) -> CyberpixieResult<ImageId>;
    /// Remove all stored images.
    ///
    /// # Notice for the board developers
    ///
    /// - You shold set the current image ID to the `None` in the board configuration.
    fn clear_images(&mut self) -> CyberpixieResult<()>;

    /// Sets an index of image that will be shown.
    fn set_current_image_id<I>(&mut self, id: I) -> CyberpixieResult<()>
    where
        I: Into<Option<ImageId>>,
    {
        let mut config = self.config()?;
        config.current_image = id.into();
        self.set_config(config)
    }

    /// Returns an index of image that will be shown.
    fn current_image_id(&self) -> CyberpixieResult<Option<ImageId>> {
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
    fn current_image(&self) -> CyberpixieResult<ImageReader<'_, Self>> {
        let image_id = self
            .current_image_id()?
            .ok_or(CyberpixieError::ImageRepositoryIsEmpty)?;
        self.read_image(image_id)
    }
}
