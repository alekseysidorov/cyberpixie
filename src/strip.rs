use smart_leds::RGB8;

use crate::{
    config::{MAX_LINES_COUNT, STRIP_LEDS_COUNT},
    time::MicroSeconds,
};

pub trait StripLineSource {
    type Line<'a>: Iterator<Item = RGB8> + 'a;

    const LINE_LENGTH: usize;

    fn next_line(&mut self) -> (MicroSeconds, Self::Line<'_>);
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
    delay: MicroSeconds,

    buf: [RGB8; FIXED_IMAGE_BUF_LEN],
}

impl FixedImage {
    pub fn from_raw(raw: &[RGB8], delay: impl Into<MicroSeconds>) -> Option<Self> {
        opt_ensure!(raw.len() <= FIXED_IMAGE_BUF_LEN, "The picture is too long.");
        opt_ensure!(
            raw.len() % STRIP_LEDS_COUNT as usize == 0,
            "The picture length must be multiple of the strip length."
        );

        let mut buf = [RGB8::default(); FIXED_IMAGE_BUF_LEN];
        buf[0..raw.len()].copy_from_slice(raw);

        Some(Self {
            current_line: 0,
            image_len: raw.len() as u16,
            buf,
            delay: delay.into(),
        })
    }
}

impl StripLineSource for FixedImage {
    type Line<'a> = impl Iterator<Item = RGB8> + 'a;

    const LINE_LENGTH: usize = STRIP_LEDS_COUNT;

    fn next_line(&mut self) -> (MicroSeconds, Self::Line<'_>) {
        let start = self.current_line as usize;
        let end = start + Self::LINE_LENGTH;
        self.current_line = (self.current_line + self.image_len) % FIXED_IMAGE_BUF_LEN as u16;

        let iter = self.buf[start..end].iter().copied();
        (self.delay, iter)
    }
}
