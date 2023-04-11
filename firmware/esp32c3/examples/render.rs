use std::time::Duration;

use anyhow::Context;
use cyberpixie_core::service::{DeviceStorage, ImageLines};
use cyberpixie_esp32c3::{render, storage::ImagesRegistry};
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const LED_PIN: u32 = 8;

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();

    let mut strip = Ws2812Esp32Rmt::new(0, LED_PIN)?;
    // Clear strip
    strip.write(std::iter::repeat(RGB8::default()).take(144))?;

    let mut storage = ImagesRegistry::new();

    let Some(image_id) = storage.current_image()? else {
        log::error!("There is no images in storage");
        return Ok(());
    };

    let image = storage.read_image(image_id)?;
    log::info!("Rendering {} image", image_id);

    let mut lines = ImageLines::new(image, storage.config()?.strip_len);
    loop {
        let (line, frequency) = lines
            .next_line()
            .context("Unable to read next image line")?;
        let duration = Duration::from_secs_f32(1.0 / frequency.0 as f32);

        strip
            .write(line.into_iter())
            .expect("Unable to show image line");
        std::thread::sleep(duration);
    }
}
