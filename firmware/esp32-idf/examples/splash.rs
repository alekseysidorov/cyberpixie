use std::time::Duration;

use cyberpixie_esp32_idf::splash::WanderingLight;
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const TICK_DELAY: u32 = 3;
const STRIP_LEN: usize = 144;
const LED_PIN: u32 = 8;

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();

    let mut strip = Ws2812Esp32Rmt::new(0, LED_PIN)?;

    // Clear strip
    strip.write(std::iter::repeat(RGB8::default()).take(144))?;

    for brightness in 16..128 {
        let splash = WanderingLight::<STRIP_LEN>::new(brightness);

        for (ticks, line) in splash {
            strip.write(line.into_iter())?;
            std::thread::sleep(Duration::from_micros((TICK_DELAY * ticks) as u64));
        }
    }

    Ok(())
}
