use smart_leds::RGB8;

const COLOR_CMD_LEN: usize = 12;
const BLANK_LINE_LEN: usize = 50;
const PATTERNS: [u8; 4] = [0b1000_1000, 0b1000_1110, 0b1110_1000, 0b1110_1110];

pub const fn size_of_line(rgb8_len: usize) -> usize {
    BLANK_LINE_LEN + COLOR_CMD_LEN * rgb8_len
}

#[inline]
pub fn make_row<const N: usize>(iter: impl IntoIterator<Item = RGB8>) -> [u8; N]
{
    let iter = iter.into_iter();

    let mut data = [0_u8; N];
    // Fill the pixel commands part of line.
    for (led_bytes, RGB8 { r, g, b }) in data.chunks_mut(COLOR_CMD_LEN).zip(iter) {
        for (i, mut color) in [r, g, b].into_iter().enumerate() {
            for ii in 0..4 {
                led_bytes[i * 4 + ii] = PATTERNS[((color & 0b1100_0000) >> 6) as usize];
                color <<= 2;
            }
        }
    }
    data
}
