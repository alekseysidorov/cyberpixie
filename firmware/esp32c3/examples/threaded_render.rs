use std::time::{Duration, Instant};

use anyhow::Context;
use cyberpixie_core::{
    proto::types::Hertz,
    service::{DeviceStorage, ImageLines},
    ExactSizeRead, MAX_STRIP_LEN,
};
use cyberpixie_esp32c3::{storage::ImagesRegistry, DEFAULT_DEVICE_CONFIG};
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
    let storage = ImagesRegistry::new(DEFAULT_DEVICE_CONFIG);
    log::info!("{:?}", storage.images_count());
    log::info!("{:?}", storage.current_image());
    let Some(image_id) = storage.current_image()? else {
        log::error!("There is no images in storage");
        return Ok(());
    };

    let image = storage.read_image(image_id)?;
    log::info!("Rendering {} image", image_id);
    log::info!("image_len: {}", image.bytes.bytes_remaining());

    let strip_len = storage.config()?.strip_len;
    let mut buf = vec![0_u8; strip_len as usize * 3];
    let mut lines = ImageLines::new(image, strip_len, &mut buf);

    let (tx, rx) = std::sync::mpsc::sync_channel::<heapless::Vec<RGB8, MAX_STRIP_LEN>>(4);
    let actual_refresh_rate = lines.refresh_rate();
    std::thread::Builder::new()
        .name("rendering".to_owned())
        .stack_size(5_000)
        .spawn(move || -> anyhow::Result<()> {
            let mut refresh_rate = Hertz(1);
            let mut check_max_rate = true;
            loop {
                let mut refresh_perion = Duration::from(refresh_rate);
                if check_max_rate {
                    log::info!(
                        "Refresh period is {} [{} Hz]",
                        refresh_perion.as_secs_f32(),
                        refresh_rate.0
                    );
                }

                let now = Instant::now();
                let line = rx.recv().context("Unable to read a next image line")?;

                let elapsed = now.elapsed();
                if elapsed > refresh_perion {
                    log::warn!(
                        "Frame reading took too much time: {}, must be lesser than {} [{} Hz]",
                        elapsed.as_secs_f32(),
                        refresh_perion.as_secs_f32(),
                        refresh_rate.0
                    );
                }

                let now2 = Instant::now();
                strip
                    .write(line.into_iter())
                    .context("Unable to show image line")?;
                let elapsed = now2.elapsed();

                if elapsed > refresh_perion {
                    log::warn!(
                        "Frame rendering took too much time: {}, must be lesser than {} [{} Hz]",
                        elapsed.as_secs_f32(),
                        refresh_perion.as_secs_f32(),
                        refresh_rate.0
                    );
                }

                if !check_max_rate {
                    refresh_perion = refresh_perion.saturating_sub(now.elapsed());
                }

                std::thread::sleep(refresh_perion);

                if check_max_rate {
                    refresh_rate.0 += 1;
                    if refresh_rate.0 >= 1500 {
                        check_max_rate = false;
                        refresh_rate = actual_refresh_rate;
                        log::info!(
                            "Setting up the refresh rate from the image source: {} Hz",
                            refresh_rate.0
                        );
                    }
                }
            }
        })
        .expect("Unable to spawn thread");

    // Reading process
    loop {
        let line = lines
            .next_line()
            .context("Unable to read next image line")?
            .collect();

        tx.send(line)
            .context("Unable to send line to the rendering thread")?;
    }
}
