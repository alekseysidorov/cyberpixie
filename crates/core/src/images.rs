use core::mem::size_of;

use cyberpixie_proto::Hertz;
use smart_leds::RGB8;

pub trait ImagesRepository {
    type Error;

    const MAX_COUNT: usize;

    type ImagePixels<'a>: Iterator<Item = RGB8> + ExactSizeIterator + Clone;

    fn add_image<I>(&mut self, data: I, refresh_rate: Hertz) -> Result<usize, Self::Error>
    where
        I: Iterator<Item = RGB8>;

    fn read_image(&mut self, index: usize) -> (Hertz, Self::ImagePixels<'_>);

    fn count(&self) -> usize;

    fn clear(&mut self) -> Result<(), Self::Error>;
}

pub struct RgbIter<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    inner: I,
}

impl<I> RgbIter<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    pub fn new(inner: I) -> Self {
        assert_eq!(
            inner.len() % size_of::<RGB8>(),
            0,
            "Iterator length must be a multiple of {}.",
            size_of::<RGB8>()
        );

        Self { inner }
    }
}

impl<I> Iterator for RgbIter<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    type Item = RGB8;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rgb_count = self.inner.len() / size_of::<RGB8>();
        (rgb_count, Some(rgb_count))
    }

    fn next(&mut self) -> Option<Self::Item> {
        let rgb = RGB8 {
            r: self.inner.next()?,
            g: self.inner.next().unwrap(),
            b: self.inner.next().unwrap(),
        };

        Some(rgb)
    }
}

impl<I> ExactSizeIterator for RgbIter<I> where I: Iterator<Item = u8> + ExactSizeIterator {}
