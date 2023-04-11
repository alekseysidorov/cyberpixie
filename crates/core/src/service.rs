//! Cyberpixie service traits

use embedded_io::blocking::{ReadExactError, Seek};
use rgb::{FromSlice, RGB8};
use serde::{Deserialize, Serialize};

use crate::{
    proto::types::{DeviceInfo, Hertz, ImageId},
    ExactSizeRead,
};

pub const MAX_STRIP_LEN: usize = 72;
const IMAGE_BUF_LEN: usize = 256;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Config {
    pub strip_len: u16,
}

#[derive(Debug)]
pub struct Image<R>
where
    R: ExactSizeRead + Seek,
{
    pub refresh_rate: Hertz,
    pub bytes: R,
}

impl<R> Image<R>
where
    R: ExactSizeRead + Seek,
{
    /// Rewind to the beginning of an image.
    pub fn rewind(&mut self) -> Result<(), R::Error> {
        self.bytes.rewind()?;
        Ok(())
    }
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
    fn images_count(&self) -> crate::Result<ImageId>;
    /// Remove all stored images.
    fn clear_images(&self) -> crate::Result<()>;
    /// Sets an index of image that will be shown.
    fn set_current_image(&self, id: ImageId) -> crate::Result<()>;
    /// Returns an index of image that will be shown.
    fn current_image(&self) -> crate::Result<Option<ImageId>>;
    /// Switches to a next image, if it reaches the last image it turns back to the first image.
    fn switch_to_next_image(&self) -> crate::Result<Option<ImageId>> {
        let Some(mut current_image) = self.current_image()? else {
            return Ok(None)
        };
        
        current_image.0 += 1;
        if current_image == self.images_count()? {
            current_image.0 = 0;
        }
        Ok(Some(current_image))
    }
}

pub type ImageLine = heapless::Vec<RGB8, MAX_STRIP_LEN>;

/// An endless iterator over the image lines, then it reaches the end of image, it rewinds to the beginning.
pub struct ImageLines<R>
where
    R: ExactSizeRead + Seek,
{
    image: Image<R>,
    current_line_buf: heapless::Vec<u8, IMAGE_BUF_LEN>,
}

impl<R> ImageLines<R>
where
    R: ExactSizeRead + Seek,
{
    /// Bytes count per single pixel.
    const BYTES_PER_PIXEL: usize = 3;

    /// Creates a new image lines iterator.
    ///
    /// # Panics
    ///
    /// - If the image length is lesser that the single strip line length
    /// - If the image length in pixels is not a multiple of the strip length
    pub fn new(image: Image<R>, strip_len: u16) -> Self {
        let strip_len: usize = strip_len.into();
        let strip_len_bytes = strip_len * Self::BYTES_PER_PIXEL;
        // Check preconditions.
        assert!(
            image.bytes.bytes_remaining() >= strip_len_bytes,
            "The given image should have at least {} bytes",
            strip_len_bytes
        );
        assert!(
            image.bytes.bytes_remaining() % strip_len_bytes == 0,
            "The length of the given image in pixels is not a multiple of the given strip length."
        );

        Self {
            image,
            current_line_buf: core::iter::repeat(0).take(strip_len_bytes).collect(),
        }
    }

    /// Reads and returns a next image line
    pub fn next_line(
        &mut self,
    ) -> Result<(ImageLine, Hertz), ReadExactError<R::Error>> {
        self.fill_next_line()?;
        let line = self.current_line_buf.as_rgb().iter().copied().collect();
        Ok((line, self.image.refresh_rate))
    }

    fn fill_next_line(&mut self) -> Result<(), ReadExactError<R::Error>> {
        // In this case we reached the end of file and have to rewind to the beginning
        if self.image.bytes.bytes_remaining() == 0 {
            self.image.bytes.rewind().map_err(ReadExactError::Other)?;
        }
        // Fill the buffer with by the bytes of the next image line
        self.image.bytes.read_exact(&mut self.current_line_buf)?;
        Ok(())
    }
}
