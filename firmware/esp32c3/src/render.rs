//! Image rendering task.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use cyberpixie_core::{
    proto::types::Hertz,
    service::{DeviceStorage, ImageLines},
    ExactSizeRead, MAX_STRIP_LEN,
};
use smart_leds::{SmartLedsWrite, RGB8};

const RENDERING_QUEUE_LEN: usize = 10;

// #[derive(Debug)]
// pub struct Render<R, D> {
//     state: State<R, D>,
// }

// impl<R, D> Render<R, D> {
//     pub fn new(render: R) -> Self {
//         Self {
//             state: State::Idle { render },
//         }
//     }
// }

// impl<R, D> Render<R, D>
// where
//     R: SmartLedsWrite<Color = RGB8> + Send + 'static,
//     D: DeviceStorage + Send + 'static,
//     R::Error: std::fmt::Debug,
// {
//     pub fn start(&mut self, device: D) -> anyhow::Result<()> {
//         // let render = match self.state {
//         //     State::Idle { render } => render,
//         //     State::Running { handle } => bail!("An error in the program logic: image rendering already started."),
//         // };

//         todo!()
//     }

//     pub fn stop(&mut self) -> anyhow::Result<D> {
//         todo!()
//     }
// }

// #[derive(Debug)]
// enum State<R, D> {
//     Idle { render: R },
//     Running { handle: Handle<R, D> },
// }

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
    refresh_rate: Hertz,
) -> anyhow::Result<Handle<R, D>>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    D: DeviceStorage + Send + 'static,
    R::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
    // <D::ImageRead as Io>::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
{
    let refresh_period = Duration::from(refresh_rate);

    let cancelled = Arc::new(AtomicBool::new(false));
    let (tx, rx) =
        std::sync::mpsc::sync_channel::<heapless::Vec<RGB8, MAX_STRIP_LEN>>(RENDERING_QUEUE_LEN);

    // Create a reading task
    let is_cancelled = cancelled.clone();
    let reading_task = std::thread::Builder::new()
        .name("reading".to_owned())
        .stack_size(7_000)
        .spawn(move || -> anyhow::Result<D> {
            {
                // Get the current image ID
                let image_id = storage
                    .current_image()?
                    .context("There is no images in storage")?;
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
            let mut total_frames = 0;
            let mut laggy_frames = 0;
            let mut max_lag = Duration::default();
            let mut max_rendering_time = Duration::default();

            // Don't start rendering thread immediately
            std::thread::sleep(Duration::from_millis(0));

            loop {
                if is_cancelled.load(Ordering::Relaxed) {
                    break;
                }

                let now = Instant::now();
                let line = rx.recv().context("Unable to read a next image line")?;
                let now2 = Instant::now();
                render
                    .write(line.into_iter())
                    .context("Unable to show image line")?;
                let rendering_time = now2.elapsed();
                let total_elapsed = now.elapsed();

                total_frames += 1;
                max_lag = std::cmp::max(max_lag, total_elapsed);
                max_rendering_time = std::cmp::max(max_rendering_time, rendering_time);
                if total_elapsed > refresh_period {
                    laggy_frames += 1;
                }

                let until_next_frame = refresh_period.saturating_sub(now.elapsed());
                std::thread::sleep(until_next_frame);
            }
            // Clear strip after rendering
            render.write(std::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))?;

            log::info!("Image rendering finished");
            log::info!("-> Laggy frames {} of {}", laggy_frames, total_frames);
            log::info!(
                "-> Max frame rendering duration is {}ms",
                max_rendering_time.as_secs_f32() * 1_000_f32
            );
            log::info!(
                "-> Max frame rendering frame rate is {}Hz",
                1.0_f32 / max_rendering_time.as_secs_f32()
            );
            log::info!(
                "-> Max total frame handling duration is {}ms",
                max_lag.as_secs_f32() * 1_000_f32
            );

            Ok(render)
        })
        .context("Unable to spawn rendering thread")?;

    Ok(Handle {
        reading_task,
        rendering_task,
        cancelled,
    })
}
