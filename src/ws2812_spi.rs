use smart_leds::RGB8;

const COLOR_CMD_LEN: usize = 12;
const BLANK_LINE_LEN: usize = 50;
const BLANK_LINE: [u8; BLANK_LINE_LEN] = [0u8; BLANK_LINE_LEN];
const PATTERNS: [u8; 4] = [0b1000_1000, 0b1000_1110, 0b1110_1000, 0b1110_1110];

pub const fn size_of_line(rgb8_len: usize) -> usize {
    BLANK_LINE_LEN + COLOR_CMD_LEN * rgb8_len
}

#[inline]
pub fn make_line(rgb: impl IntoIterator<Item = RGB8>) -> impl Iterator<Item = u8> {
    rgb.into_iter()
        .flat_map(|pixel| {
            let mut led_bytes = [0_u8; COLOR_CMD_LEN];

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
