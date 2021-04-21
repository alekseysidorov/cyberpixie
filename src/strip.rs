use smart_leds::RGB8;

use crate::config::{MAX_LINES_COUNT, STRIP_LEDS_COUNT};

pub trait StripLineSource {
    type Line<'a>: Iterator<Item = RGB8> + 'a;

    const LINE_LENGTH: usize;

    fn next_line(&mut self) -> Self::Line<'_>;
}

const FIXED_IMAGE_BUF_LEN: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;

macro_rules! opt_ensure {
    ($e:expr, $msg:expr) => {
        if !($e) {
            $crate::uprintln!($msg);
            return None;
        }
    };
}

/// Fixed image.
#[derive(Debug)]
pub struct FixedImage {
    image_len: u16,
    current_line: u16,
    buf: [RGB8; FIXED_IMAGE_BUF_LEN],
}

impl FixedImage {
    pub fn empty() -> Self {
        Self {
            current_line: 0,
            image_len: 0,
            buf: [RGB8::default(); FIXED_IMAGE_BUF_LEN],
        }
    }

    pub fn from_data(data: &[RGB8]) -> Option<Self> {
        let mut img = Self::empty();
        img.reset(data)?;
        Some(img)
    }

    pub fn reset(&mut self, data: &[RGB8]) -> Option<()> {
        opt_ensure!(
            data.len() <= FIXED_IMAGE_BUF_LEN,
            "The picture is too long."
        );
        opt_ensure!(
            data.len() % STRIP_LEDS_COUNT as usize == 0,
            "The picture length must be multiple of the strip length."
        );

        self.buf[0..data.len()].copy_from_slice(data);
        self.image_len = data.len() as u16;
        self.current_line = 0;

        Some(())
    }

    pub fn height(&self) -> u16 {
        self.image_len / Self::LINE_LENGTH as u16
    }
}

impl StripLineSource for FixedImage {
    type Line<'a> = impl Iterator<Item = RGB8> + 'a;

    const LINE_LENGTH: usize = STRIP_LEDS_COUNT;

    fn next_line(&mut self) -> Self::Line<'_> {
        let start = self.current_line as usize;
        let end = start + Self::LINE_LENGTH;
        self.current_line = (self.current_line + Self::LINE_LENGTH as u16) % self.image_len;

        self.buf[start..end].iter().copied()
    }
}
