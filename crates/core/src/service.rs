//! Cyberpixie service traits

use embedded_io::blocking::{ReadExactError, Seek};
use rgb::{FromSlice, RGB8};
use serde::{Deserialize, Serialize};

use crate::{
    proto::types::{DeviceInfo, Hertz, ImageId},
    ExactSizeRead,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceConfig {
    /// The number of LEDs in the strip.
    pub strip_len: u16,
}

#[derive(Debug)]
pub struct Image<R>
where
    R: ExactSizeRead + Seek,
{
    /// Refresh rate of the square area of picture with the strip length size.
    ///
    /// That is, the refresh rate of a single line of a picture is the refresh rate of
    /// the entire image multiplied by the strip length.
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

/// Image rendering handle.
pub trait ImageRenderHandle {
    /// Stops an image rendering routine.
    fn stop(self);
}

/// Basic device services.
pub trait DeviceService {
    /// Device storage type.
    type Storage: DeviceStorage;
    /// Image rendering handle.
    type ImageRender;
    /// Returns important device information necessary for handshake.
    fn device_info(&self) -> DeviceInfo;
    /// Returns handle to the device storage.
    fn storage(&self) -> Self::Storage;
    /// Starts a current image rendering.
    fn show_current_image(&mut self) -> Self::ImageRender;
}

/// A type definition to represent an image reader for a certain device.
pub type DeviceImage<'a, S> = Image<<S as DeviceStorage>::ImageRead<'a>>;

/// Device data storage.
pub trait DeviceStorage {
    type ImageRead<'a>: ExactSizeRead + Seek
    where
        Self: 'a;

    /// Returns a global configuration.
    fn config(&self) -> crate::Result<DeviceConfig>;
    /// Sets a global configuration.
    fn set_config(&self, value: &DeviceConfig) -> crate::Result<()>;
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

/// An endless iterator over the image lines, then it reaches the end of image, it rewinds to the beginning.
pub struct ImageLines<R, B>
where
    B: AsMut<[u8]>,
    R: ExactSizeRead + Seek,
{
    image: Image<R>,
    strip_line_len: usize,
    strip_line_buf: B,
    refresh_rate: Hertz,
}

impl<R, B> ImageLines<R, B>
where
    B: AsMut<[u8]>,
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
    pub fn new(image: Image<R>, strip_len: u16, mut strip_line_buf: B) -> Self {
        let strip_len: usize = strip_len.into();
        let strip_line_len = strip_len * Self::BYTES_PER_PIXEL;
        // Check preconditions.
        assert!(
            image.bytes.bytes_remaining() >= strip_line_len,
            "The given image should have at least {} bytes",
            strip_line_len
        );
        assert!(
            image.bytes.bytes_remaining() % strip_line_len == 0,
            "The length of the given image in pixels `{}` is not a multiple of the given strip length `{}`.",
            image.bytes.bytes_remaining(),
            strip_line_len
        );
        assert!(
            strip_line_buf.as_mut().len() >= strip_line_len,
            "Given buffer capacity is not enough"
        );

        // Compute the single line refresh rate.
        let refresh_rate = Hertz(image.refresh_rate.0 * strip_len as u32);

        Self {
            image,
            strip_line_len,
            strip_line_buf,
            refresh_rate,
        }
    }

    /// Returns a refresh line fo the single strip line.
    ///
    /// We assume that the refresh rate in the given image if the frequency of
    /// redrawing of the square area of the picture with the strip lenght size.
    pub fn refresh_rate(&self) -> Hertz {
        self.refresh_rate
    }

    /// Reads and returns a next image line
    pub fn next_line(
        &mut self,
    ) -> Result<impl Iterator<Item = RGB8> + '_, ReadExactError<R::Error>> {
        let line = self.fill_next_line()?.as_rgb().iter().copied();
        Ok(line)
    }

    fn fill_next_line(&mut self) -> Result<&[u8], ReadExactError<R::Error>> {
        // In this case we reached the end of file and have to rewind to the beginning
        if self.image.bytes.bytes_remaining() == 0 {
            self.image.bytes.rewind().map_err(ReadExactError::Other)?;
        }
        // Fill the buffer with by the bytes of the next image line
        let buf = &mut self.strip_line_buf.as_mut()[0..self.strip_line_len];
        self.image.bytes.read_exact(buf)?;
        Ok(buf)
    }
}
