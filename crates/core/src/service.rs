//! Cyberpixie service traits

use embedded_io::blocking::{ReadExactError, Seek};
use rgb::{FromSlice, RGB8};
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
    fn images_count(&self) -> crate::Result<u16>;
    /// Remove all stored images.
    fn clear_images(&self) -> crate::Result<()>;
}

/// An endless iterator over the image lines, then it reaches the end of image, it rewinds to the beginning.
pub struct ImageLines<R, const BUF_LEN: usize = 600>
where
    R: ExactSizeRead + Seek,
{
    image: Image<R>,
    current_line_buf: heapless::Vec<u8, BUF_LEN>,
}

impl<R, const BUF_LEN: usize> ImageLines<R, BUF_LEN>
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
    ) -> Result<(impl Iterator<Item = RGB8> + '_, Hertz), ReadExactError<R::Error>> {
        self.fill_next_line()?;
        let line = self.current_line_buf.as_rgb().iter().copied();
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
