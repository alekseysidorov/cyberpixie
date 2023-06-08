//! Strip LED picture render

use cyberpixie_app::{
    core::{
        io::image_reader::ImageLines,
        proto::types::{Hertz, ImageId},
        MAX_STRIP_LEN,
    },
    Storage,
};
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embassy_time::{Duration, Instant, Timer};
use smart_leds::RGB8;
use ws2812_async::Ws2812;

use crate::{singleton, SpiType, StorageImpl};

/// Pending frames queue length.
const QUEUE_LEN: usize = 8;

pub type RGB8Line = heapless::Vec<RGB8, MAX_STRIP_LEN>;
pub type StaticSender<T, const N: usize> = Sender<'static, CriticalSectionRawMutex, T, N>;
pub type StaticReceiver<T, const N: usize> = Receiver<'static, CriticalSectionRawMutex, T, N>;

enum Command {
    Start { storage: StorageImpl, id: ImageId },
    Stop,
}

/// Next frame
pub enum Frame {
    /// Change refresh frame rate.
    UpdateRate(Hertz),
    /// Next line.
    Line(RGB8Line),
    /// Cleanup the strip.
    Clear,
}

#[embassy_executor::task]
async fn storage_reading_task(
    commands: StaticReceiver<Command, 1>,
    responses: StaticSender<StorageImpl, 1>,
    framebuffer: StaticSender<Frame, QUEUE_LEN>,
) {
    let mut pending: Option<(StorageImpl, ImageId)> = None;
    loop {
        if let Some((mut storage, id)) = pending.take() {
            // There is a received picture rendering task.
            let strip_len = storage.config().unwrap().strip_len;

            let mut reader = ImageLines::new(
                storage.read_image(id).unwrap(),
                strip_len,
                [0_u8; MAX_STRIP_LEN * 3],
            );
            let rate = reader.refresh_rate();
            log::info!(
                "Starting a new picture rendering task id: {}, rate: {}Hz",
                id,
                rate
            );
            // Start an endless loop of reading and sending frames to the rendering task,
            // which can be stopped by the `Stop` command.
            framebuffer.send(Frame::UpdateRate(rate)).await;
            loop {
                let line: RGB8Line = reader.next_line().unwrap().collect();
                // Send line to the rendering thread.
                framebuffer.send(Frame::Line(line)).await;
                // Check if a stop command has been sent.
                if let Ok(Command::Stop) = commands.try_recv() {
                    // Stop this rendering task
                    log::info!("Stopping a rendering task");
                    break;
                }
            }
            // Cleanup strip.
            framebuffer.send(Frame::Clear).await;
            // After stopping the picture read cycle, we should return the storage instance back
            // to the main application.
            responses.send(storage).await;
            log::info!("Rendering task stopped");
        } else {
            // Waiting for a new rendering task.
            let Command::Start { storage, id } = commands.recv().await else { continue; };
            pending.replace((storage, id));
            log::info!("Received a new picture rendering task");
        }
    }
}

#[embassy_executor::task]
pub async fn render_task(
    spi: &'static mut SpiType<'static>,
    receiver: StaticReceiver<Frame, QUEUE_LEN>,
) {
    const LED_BUF_LEN: usize = 12 * MAX_STRIP_LEN;

    // Initialize and cleanup a LEN strip.
    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);
    ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
        .await
        .unwrap();

    // Default frame duration
    let mut rate = Hertz(500);
    let mut frame_duration = Duration::from_hz(rate.0 as u64);

    let mut total_render_time = 0;
    let mut dropped_frames = 0;
    let mut counts = 0;
    let mut max_render_time = 0;
    loop {
        let now = Instant::now();
        match receiver.recv().await {
            // Received a new picture frame rate, we should update a refresh period and wait for
            // a short time until the frames queue will be fill.
            Frame::UpdateRate(new_rate) => {
                rate = new_rate;
                frame_duration = Duration::from_hz(rate.0 as u64);
                Timer::after(frame_duration * QUEUE_LEN as u32 * 2).await;
            }

            Frame::Line(line) => {
                ws.write(line.into_iter()).await.unwrap();
                let elapsed = now.elapsed();

                total_render_time += elapsed.as_micros();
                if elapsed <= frame_duration {
                    let next_frame_time = now + frame_duration;
                    Timer::at(next_frame_time).await;
                } else {
                    dropped_frames += 1;
                }
                max_render_time = core::cmp::max(max_render_time, elapsed.as_micros());
                counts += 1;
            }

            Frame::Clear => {
                ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
                    .await
                    .unwrap();
                // Reset rendering stats.
                dropped_frames = 0;
                total_render_time = 0;
                max_render_time = 0;
                counts = 0;
            }
        };

        if counts == 10_000 {
            let line_render_time = total_render_time as f32 / counts as f32;
            log::info!("-> Refresh rate {rate}hz");
            log::info!("-> Total rendering time {total_render_time}us");
            log::info!("-> per line: {line_render_time}us");
            log::info!("-> max: {max_render_time}us");
            log::info!(
                "-> Average frame rendering frame rate is {}Hz",
                1_000_000f32 / line_render_time
            );
            log::info!(
                "-> dropped frames: {dropped_frames} [{}%]",
                dropped_frames as f32 * 100_f32 / counts as f32
            );

            dropped_frames = 0;
            total_render_time = 0;
            counts = 0;
        }
    }
}

/// Pictures rendering handle to control the rendering process.
#[derive(Clone)]
pub struct RenderingHandle {
    commands: StaticSender<Command, 1>,
    responses: StaticReceiver<StorageImpl, 1>,
}

impl RenderingHandle {
    pub async fn start(&self, storage: StorageImpl, id: ImageId) {
        self.commands.send(Command::Start { storage, id }).await
    }

    pub async fn stop(&self) -> StorageImpl {
        // Send stop command.
        self.commands.send(Command::Stop).await;
        // Wait for the response with the storage.
        self.responses.recv().await
    }
}

/// Creates a pictures render tasks set.
pub fn spawn(spawner: Spawner) -> (StaticReceiver<Frame, QUEUE_LEN>, RenderingHandle) {
    // Create communication channels between tasks.
    let commands = singleton!(Channel::new());
    let responses = singleton!(Channel::new());
    let framebuffer = singleton!(Channel::new());
    // Spawn Embassy tasks.
    spawner.must_spawn(storage_reading_task(
        commands.receiver(),
        responses.sender(),
        framebuffer.sender(),
    ));
    (
        framebuffer.receiver(),
        RenderingHandle {
            commands: commands.sender(),
            responses: responses.receiver(),
        },
    )
}
