use smart_leds::RGB8;

pub const BLANK_LINE_LEN: usize = 140;

pub const BLANK_LINE: [u8; BLANK_LINE_LEN] = [0u8; BLANK_LINE_LEN];

const PATTERNS: [u8; 4] = [0b1000_1000, 0b1000_1110, 0b1110_1000, 0b1110_1110];

type SpiChunk = [u8; 12];

#[inline]
pub fn rgb8_line_to_spi(rgb: impl IntoIterator<Item = RGB8>) -> impl Iterator<Item = u8> {
    rgb.into_iter()
        .flat_map(|pixel| {
            let mut led_bytes = SpiChunk::default();
            for (i, mut color) in pixel.iter().enumerate() {
                for j in 0..4 {
                    let pattern = ((color & 0b1100_0000) >> 6) as usize;
                    led_bytes[i * 4 + j] = PATTERNS[pattern];
                    color <<= 2;
                }
            }

            led_bytes
        })
        .chain(BLANK_LINE)
}
