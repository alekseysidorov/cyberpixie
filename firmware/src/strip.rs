use core::{array::IntoIter, convert::TryInto};

use smart_leds::RGB8;

use crate::{
    config::{MAX_LINES_COUNT, STRIP_LEDS_COUNT},
    time::Microseconds,
};

pub trait StripLineSource {
    const LINE_LENGTH: usize;

    type Iter: IntoIterator<Item = RGB8>;

    fn next_line(&mut self) -> (Microseconds, Self::Iter);
}

const FIXED_IMAGE_BUF_LEN: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;

/// Fixed image.
#[derive(Debug)]
pub struct FixedImage {
    image_len: u16,
    current_line: u16,
    duration: Microseconds,

    buf: [RGB8; FIXED_IMAGE_BUF_LEN],
}

impl FixedImage {
    pub fn from_raw<I, T>(raw: I, duration: T) -> Self
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
        T: Into<Microseconds>,
    {
        let image_len = raw.len();

        assert!(
            image_len <= FIXED_IMAGE_BUF_LEN,
            "The picture is too long: {} > {}",
            image_len,
            FIXED_IMAGE_BUF_LEN
        );
        assert!(
            image_len % STRIP_LEDS_COUNT as usize == 0,
            "The picture length must be multiple of the strip length."
        );

        let mut buf = [RGB8::default(); FIXED_IMAGE_BUF_LEN];
        for (idx, color) in raw.enumerate() {
            buf[idx] = color;
        }

        // Calculate the duration of the glow of a single strip.
        let height = image_len / Self::LINE_LENGTH;
        let mut duration = duration.into();
        duration.0 /= height as u32;
        assert!(
            duration.0 > 1,
            "Delay should be greater than the one microsecond."
        );

        Self {
            current_line: 0,
            image_len: image_len as u16,
            buf,
            duration,
        }
    }

    pub fn height(&self) -> u16 {
        self.image_len / Self::LINE_LENGTH as u16
    }
}

impl StripLineSource for FixedImage {
    const LINE_LENGTH: usize = STRIP_LEDS_COUNT;
    type Iter = IntoIter<RGB8, { Self::LINE_LENGTH }>;

    fn next_line(&mut self) -> (Microseconds, Self::Iter) {
        let start = self.current_line as usize;
        let end = start + Self::LINE_LENGTH;
        self.current_line = (self.current_line + Self::LINE_LENGTH as u16) % self.image_len;

        let buf: [RGB8; Self::LINE_LENGTH] = self.buf[start..end].as_ref().try_into().unwrap();
        (self.duration, IntoIter::new(buf))
    }
}