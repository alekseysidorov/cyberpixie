//! Image reader wrapper over block reader.

use embedded_io::{
    blocking::{Read, ReadExactError, Seek},
    Io, SeekFrom,
};
use rgb::{FromSlice, RGB8};

use super::{BlockReader, DEFAULT_BLOCK_SIZE};
use crate::{proto::types::Hertz, ExactSizeRead};

#[derive(Debug)]
pub struct ImageReader<T, B, const N: usize = DEFAULT_BLOCK_SIZE>
where
    T: BlockReader<N>,
{
    block_reader: T,

    image_len: usize,
    bytes_read: usize,

    block: B,
}

impl<T, const N: usize> ImageReader<T, [u8; N], N>
where
    T: BlockReader<N>,
{
    pub const fn new_in_array(block_reader: T, image_len: usize) -> Self {
        Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block: [0_u8; N],
        }
    }
}

impl<T, B, const N: usize> ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    pub fn new(block_reader: T, image_len: usize, mut block: B) -> Self {
        assert!(
            block.as_mut().len() >= N,
            "Given buffer has not enough capacity to store the entire block content"
        );

        Self {
            block_reader,
            image_len,
            bytes_read: 0,
            block,
        }
    }

    const fn current_block(&self) -> usize {
        self.bytes_read / N
    }

    fn read_current_block_to_buf(&mut self) -> Result<(), T::Error> {
        let to = core::cmp::min(self.bytes_remaining(), N);
        let block = self.current_block();

        let buf = &mut self.block.as_mut()[0..to];
        self.block_reader.read_block(block, buf)?;
        Ok(())
    }
}

impl<T: BlockReader<N>, B, const N: usize> Io for ImageReader<T, B, N> {
    type Error = T::Error;
}

impl<T, B, const N: usize> ExactSizeRead for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    fn bytes_remaining(&self) -> usize {
        self.image_len - self.bytes_read
    }
}

impl<T, B, const N: usize> Seek for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    fn seek(&mut self, seek: SeekFrom) -> Result<u64, Self::Error> {
        // Compute a new image read position
        self.bytes_read = match seek {
            #[allow(clippy::cast_possible_truncation)]
            SeekFrom::Start(pos) => pos as usize,
            // In this project, we only have to read an image from the beginning,
            // so we don't need to implement the whole seek functionality
            SeekFrom::Current(_pos) => unimplemented!(),
            SeekFrom::End(_pos) => unimplemented!(),
        };
        // Reread the block corresponding to the new read position
        self.read_current_block_to_buf()?;
        Ok(self.bytes_read as u64)
    }
}

impl<T, B, const N: usize> Read for ImageReader<T, B, N>
where
    T: BlockReader<N>,
    B: AsMut<[u8]>,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Just return if there is nothing to read or given buffer has zero size.
        if self.is_empty() || buf.is_empty() {
            return Ok(0);
        }
        // Fill block buffer if we are on the edge between blocks.
        if self.bytes_read % N == 0 {
            self.read_current_block_to_buf()?;
        }

        // Compute how much bytes we can copy to the outgoing buffer.
        let max_bytes_to_read = core::cmp::min(self.bytes_remaining(), buf.len());
        // Compute the position from where we should copy bytes from the block buffer.
        let from = self.bytes_read % N;
        // We can read the maximum number of bytes but no more than the length
        // of the remaining block part.
        let bytes_to_read = core::cmp::min(max_bytes_to_read, N - from);
        let to = from + bytes_to_read;
        // Perform copying bytes from block to outgoing buffer.
        buf[0..bytes_to_read].copy_from_slice(&self.block.as_mut()[from..to]);
        // Increment block read position
        self.bytes_read += bytes_to_read;
        Ok(bytes_to_read)
    }
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
    R: ExactSizeRead + Seek,
{
    image: Image<R>,
    strip_line_len: usize,
    strip_line_buf: B,
}

impl<R, B> ImageLines<R, B>
where
    B: AsMut<[u8]>,
    R: ExactSizeRead + Seek,
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
