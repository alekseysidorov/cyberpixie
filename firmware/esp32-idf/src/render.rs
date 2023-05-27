//! Image rendering task.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use cyberpixie_app::Storage;
use cyberpixie_core::{
    proto::types::{Hertz, ImageId},
    ExactSizeRead, MAX_STRIP_LEN, storage::ImageLines
};
use smart_leds::{SmartLedsWrite, RGB8};

const RENDERING_QUEUE_LEN: usize = 30;

/// Approximated duration of single pixel rendering.
const PIXEL_RENDERING_DURATION: Duration = Duration::from_nanos(34722);

#[derive(Debug)]
pub struct Handle<R, D> {
    reading_task: std::thread::JoinHandle<anyhow::Result<D>>,
    rendering_task: std::thread::JoinHandle<anyhow::Result<R>>,
    cancelled: Arc<AtomicBool>,
}

impl<R, D> Handle<R, D> {
    pub fn stop(self) -> anyhow::Result<(R, D)> {
        // Cancel tasks
        self.cancelled.store(true, Ordering::Relaxed);
        // Wait until tasks will be finished.
        let render = self
            .rendering_task
            .join()
            .expect("Unable to fisish an image rendering thread")?;
        let device = self
            .reading_task
            .join()
            .expect("Unable to finish an image reading thread")?;
        Ok((render, device))
    }
}

pub fn start_rendering<R, D>(
    mut render: R,
    storage: D,
    image_id: ImageId,
    refresh_rate: Hertz,
) -> anyhow::Result<Handle<R, D>>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    D: Storage + Send + 'static,
    R::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
    // <D::ImageRead as Io>::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
{
    let refresh_period = Duration::from(refresh_rate);
    let strip_len = storage.config()?.strip_len;

    let cancelled = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<RGB8>>(RENDERING_QUEUE_LEN);

    // Create a reading task
    let is_cancelled = cancelled.clone();
    let reading_task = std::thread::Builder::new()
        .name("reading".to_owned())
        .stack_size(7_000)
        .spawn(move || -> anyhow::Result<D> {
            {
                // Create image reader
                let image = storage.read_image(image_id)?;
                log::info!("Rendering image with index: {image_id}");
                log::info!("image_len: {}", image.bytes.bytes_remaining());
                // Create image lines iterator.
                let strip_len = storage.config()?.strip_len;
                let buf = vec![0_u8; strip_len as usize * 3];
                let mut lines = ImageLines::new(image, strip_len, buf);

                log::info!("refresh_rate: {refresh_rate}Hz");
                log::info!(
                    "refresh_period: {}ms",
                    refresh_period.as_secs_f32() * 1_000_f32
                );

                let mut total_reading_time = Duration::default();
                let mut max_reading_time = Duration::default();
                let mut total_read_frames = 0;
                loop {
                    if is_cancelled.load(Ordering::Relaxed) {
                        break;
                    }

                    let now = Instant::now();
                    let line = lines
                        .next_line()
                        .expect("Unable to send next image line")
                        .collect();
                    let elapsed = now.elapsed();
                    max_reading_time = std::cmp::max(max_reading_time, elapsed);
                    total_reading_time += elapsed;
                    total_read_frames += 1;

                    let _ok = tx.send(line);
                }

                let avg_reading_time = total_reading_time / total_read_frames;
                log::info!("Image reading finished");
                log::info!(
                    "-> Average frame reading duration is {}ms",
                    avg_reading_time.as_secs_f32() * 1_000_f32
                );
                log::info!(
                    "-> Average frame reading rate is {}Hz",
                    1.0_f32 / avg_reading_time.as_secs_f32()
                );
                log::info!(
                    "-> Max frame reading duration is {}ms",
                    max_reading_time.as_secs_f32() * 1_000_f32
                );
            }

            Ok(storage)
        })
        .context("Unable to spawn reading thread")?;

    // Create a rendering task
    let is_cancelled = cancelled.clone();
    let rendering_task = std::thread::Builder::new()
        .name("rendering".to_owned())
        .stack_size(5_000)
        .spawn(move || -> anyhow::Result<R> {
            // Don't start rendering thread immediately
            std::thread::sleep(Duration::from_millis(0));

            let critical_section = esp_idf_hal::task::CriticalSection::new();
            let expected_rendering_time = PIXEL_RENDERING_DURATION * u32::from(strip_len);

            let mut stats = RenderingStats::new(refresh_period, expected_rendering_time);
            loop {
                if is_cancelled.load(Ordering::Relaxed) {
                    break;
                }

                let rendering_time = {
                    let _guard = critical_section.enter();

                    let now = Instant::now();
                    let line = rx.recv().context("Unable to read a next image line")?;
                    render
                        .write(line.into_iter())
                        .context("Unable to show image line")?;
                    now.elapsed()
                };

                // FIXME Don't rely on the inaccurate FreeRTOS system timer.
                // let remaining_delay = refresh_period - rendering_time;
                // Rely just on the theoretical expected rendering duration.
                let remaining_delay = refresh_period - expected_rendering_time;
                esp_idf_hal::delay::Ets::delay_ms(remaining_delay.as_millis() as u32);

                stats.update(rendering_time);
                if stats.total_frames % 10_000 == 0 {
                    stats.show();
                }
            }
            // Clear strip after rendering
            render.write(std::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))?;

            log::info!("Image rendering finished");
            stats.show();

            Ok(render)
        })
        .context("Unable to spawn rendering thread")?;

    Ok(Handle {
        reading_task,
        rendering_task,
        cancelled,
    })
}

#[derive(Default)]
struct RenderingStats {
    refresh_period: Duration,
    expected_rendering_time: Duration,

    total_frames: usize,
    laggy_frames: usize,
    max_rendering_time: Duration,
    total_rendering_time: Duration,
}

impl RenderingStats {
    fn new(refresh_period: Duration, expected_rendering_time: Duration) -> Self {
        Self {
            refresh_period,
            expected_rendering_time,
            ..Default::default()
        }
    }

    fn show(&self) {
        log::info!("Print statistics snapshot");
        log::info!(
            "-> Laggy frames {} of {} [{}%]",
            self.laggy_frames,
            self.total_frames,
            (self.laggy_frames as f64 / self.total_frames as f64 * 100_f64)
        );
        log::info!(
            "-> Max frame rendering duration is {}ms",
            self.max_rendering_time.as_secs_f32() * 1_000_f32
        );
        log::info!(
            "-> Max frame rendering frame rate is {}Hz",
            1.0_f32 / self.max_rendering_time.as_secs_f32()
        );

        let average_rendering_time = self.total_rendering_time / self.total_frames as u32;
        log::info!(
            "-> Average frame rendering duration is {}ms",
            average_rendering_time.as_secs_f32() * 1_000_f32
        );
        log::info!(
            "-> Average frame rendering frame rate is {}Hz",
            1.0_f32 / average_rendering_time.as_secs_f32()
        );

        log::info!(
            "-> Expected frame rendering duration is {}ms",
            self.expected_rendering_time.as_secs_f32() * 1_000_f32
        );
        log::info!(
            "-> Expected frame rendering frame rate is {}Hz",
            1.0_f32 / self.expected_rendering_time.as_secs_f32()
        );
    }

    fn update(&mut self, rendering_time: Duration) {
        self.total_frames += 1;
        self.max_rendering_time = std::cmp::max(self.max_rendering_time, rendering_time);
        self.total_rendering_time += rendering_time;
        if rendering_time > self.refresh_period {
            self.laggy_frames += 1;
        }
    }
}
