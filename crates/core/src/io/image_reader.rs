//! Image reader wrapper raw embedded I/O reader.

use embedded_io::blocking::ReadExactError;
use rgb::{FromSlice, RGB8};

use crate::{
    io::{BlockingRead, BlockingSeek},
    proto::types::Hertz,
    ExactSizeRead,
};

#[derive(Debug)]
pub struct Image<R> {
    /// Refresh rate of the square area of picture with the strip length size.
    ///
    /// That is, the refresh rate of a single line of a picture is the refresh rate of
    /// the entire image multiplied by the strip length.
    pub refresh_rate: Hertz,
    pub bytes: R,
}

impl<R> Image<R>
where
    R: BlockingSeek,
{
    /// Rewind to the beginning of an image.
    #[inline]
    pub fn rewind(&mut self) -> Result<(), R::Error> {
        self.bytes.rewind()?;
        Ok(())
    }
}

/// An endless iterator over the image lines, then it reaches the end of image, it rewinds to the beginning.
pub struct ImageLines<R, B>
where
    B: AsMut<[u8]>,
{
    image: Image<R>,
    strip_line_len: usize,
    strip_line_buf: B,
}

impl<R, B> ImageLines<R, B>
where
    B: AsMut<[u8]>,
    R: BlockingRead + BlockingSeek + ExactSizeRead,
{
    /// Bytes count per single pixel.
    pub const BYTES_PER_PIXEL: usize = 3;

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
            "The given image should have at least {strip_line_len} bytes"
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

        Self {
            image,
            strip_line_len,
            strip_line_buf,
        }
    }

    /// Returns a refresh line fo the single strip line.
    pub const fn refresh_rate(&self) -> Hertz {
        self.image.refresh_rate
    }

    /// Reads and returns a next image line.
    #[inline]
    pub fn next_line(
        &mut self,
    ) -> Result<impl Iterator<Item = RGB8> + '_, ReadExactError<R::Error>> {
        let line = self.fill_next_line()?.as_rgb().iter().copied();
        Ok(line)
    }

    #[inline]
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
