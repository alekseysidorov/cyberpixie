use std::time::Duration;

use cyberpixie_app::{core::proto::types::Hertz, Storage};
use cyberpixie_esp32_idf::{storage::ImagesRegistry, DEFAULT_DEVICE_CONFIG};
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

    // Initialize and clear strip
    let mut strip = Ws2812Esp32Rmt::new(0, LED_PIN)?;
    strip.write(std::iter::repeat(RGB8::default()).take(144))?;
    // Initialize storage
    let mut storage = ImagesRegistry::new(DEFAULT_DEVICE_CONFIG);

    let mut refresh_rate = Hertz(50);
    let mut render = Some(strip);
    loop {
        // Render a current image.
        let handle = cyberpixie_esp32_idf::render::start_rendering(
            render.take().unwrap(),
            storage,
            storage.current_image_id()?.unwrap(),
            refresh_rate,
        )?;
        // Wait for a half minute
        std::thread::sleep(Duration::from_secs(30));
        // Finish rendering task and swith to a next stored image.
        render = Some(handle.stop()?.0);
        storage.switch_to_next_image()?;
        refresh_rate.0 += 50;
        log::info!("Next image");
    }
}
