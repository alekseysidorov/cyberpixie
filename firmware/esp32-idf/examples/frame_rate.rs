use std::time::{Duration, Instant};

use cyberpixie_app::core::proto::types::Hertz;
use cyberpixie_esp32_idf::splash::WanderingLight;
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;

const ITERS_NUM: usize = 1;
const STRIP_LEN: usize = 24;
const LED_PIN: u32 = 8;

const PIXEL_RENDERING_DURATION: Duration = Duration::from_nanos(34722);

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    EspLogger::initialize_default();

    let mut strip = Ws2812Esp32Rmt::new(0, LED_PIN)?;
    // Clear strip
    strip.write(std::iter::repeat(RGB8::default()).take(144))?;

    let splash = WanderingLight::<STRIP_LEN>::new(16);
    let mut refresh_rate = Hertz(100);
    let expected_rendering_time = PIXEL_RENDERING_DURATION * STRIP_LEN as u32;
    while refresh_rate.0 <= 1200 {
        let mut total_frames = 0;
        let mut laggy_frames = 0;
        let mut max_lag = Duration::default();

        let refresh_period = Duration::from(refresh_rate);
        log::info!("Rendering cycle started");
        log::info!("-> Refresh rate {}Hz", refresh_rate);
        log::info!(
            "-> Refresh period {}ms",
            refresh_period.as_secs_f32() * 1_000_f32
        );
        for _ in 0..ITERS_NUM {
            for (_ticks, line) in splash.clone() {
                let now = Instant::now();
                strip.write(line.into_iter())?;
                let elapsed = now.elapsed();

                max_lag = std::cmp::max(max_lag, elapsed);
                if elapsed >= refresh_period {
                    laggy_frames += 1;
                }

                total_frames += 1;
                let until_next_frame = refresh_period.saturating_sub(now.elapsed());
                std::thread::sleep(until_next_frame);
            }
        }

        log::info!("Rendering cycle finished");
        log::info!("-> Laggy frames {} of {}", laggy_frames, total_frames);
        log::info!(
            "-> Max frame rendering duration is {}ms",
            max_lag.as_secs_f32() * 1_000_f32
        );
        log::info!(
            "-> Max frame rendering frame rate is {}Hz",
            1.0_f32 / max_lag.as_secs_f32()
        );
        log::info!(
            "-> Expected frame rendering duration is {}ms",
            expected_rendering_time.as_secs_f32() * 1_000_f32
        );
        log::info!(
            "-> Expected frame rendering frame rate is {}Hz",
            1.0_f32 / expected_rendering_time.as_secs_f32()
        );

        refresh_rate.0 += 50;
    }

    Ok(())
}
