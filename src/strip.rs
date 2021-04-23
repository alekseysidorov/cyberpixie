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

macro_rules! opt_ensure {
    ($e:expr, $_msg:expr) => {
        if !($e) {
            return None;
        }
    };
}

/// Fixed image.
#[derive(Debug)]
pub struct FixedImage {
    image_len: u16,
    current_line: u16,
    duration: Microseconds,

    buf: [RGB8; FIXED_IMAGE_BUF_LEN],
}

impl FixedImage {
    pub fn from_raw<I>(raw: &[RGB8], duration: I) -> Option<Self>
    where
        I: Into<Microseconds>,
    {
        opt_ensure!(raw.len() <= FIXED_IMAGE_BUF_LEN, "The picture is too long.");
        opt_ensure!(
            raw.len() % STRIP_LEDS_COUNT as usize == 0,
            "The picture length must be multiple of the strip length."
        );

        let mut buf = [RGB8::default(); FIXED_IMAGE_BUF_LEN];
        buf[0..raw.len()].copy_from_slice(raw);

        // Calculate the duration of the glow of a single strip.
        let height = raw.len() / Self::LINE_LENGTH;
        let mut duration = duration.into();
        duration.0 /= height as u32;
        opt_ensure!(
            duration.0 > 1,
            "Delay should be greater than the one microsecond."
        );

        Some(Self {
            current_line: 0,
            image_len: raw.len() as u16,
            buf,
            duration,
        })
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
