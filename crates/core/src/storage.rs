use core::mem::size_of;

use cyberpixie_proto::Hertz;
use serde::{Deserialize, Serialize};
use smart_leds::RGB8;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    pub current_image_index: u16,
    pub strip_len: u16,
    pub receiver_buf_capacity: usize,
}

pub trait Storage {
    type Error;

    const MAX_IMAGES_COUNT: usize;

    type ImagePixels<'a>: Iterator<Item = RGB8> + ExactSizeIterator + Clone;

    fn add_image<I>(&self, data: I, refresh_rate: Hertz) -> Result<usize, Self::Error>
    where
        I: Iterator<Item = RGB8>;

    fn read_image(&self, index: usize) -> (Hertz, Self::ImagePixels<'_>);

    fn images_count(&self) -> usize;

    fn clear_images(&self) -> Result<(), Self::Error>;

    fn load_config(&self) -> Result<AppConfig, Self::Error>;

    fn save_config(&self, cfg: &AppConfig) -> Result<(), Self::Error>;
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
