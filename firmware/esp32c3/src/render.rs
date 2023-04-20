//! Image rendering task.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};

use cyberpixie_core::{
    proto::types::{Hertz, ImageId},
    service::{DeviceStorage, ImageLine, ImageLines},
};
use smart_leds::{SmartLedsWrite, RGB8};

pub struct Handle<R, D> {
    render_image_thread: std::thread::JoinHandle<(R, D)>,
    cancelled: Arc<AtomicBool>,
}

impl<R, D> Handle<R, D> {
    pub fn stop(self) -> anyhow::Result<(R, D)> {
        self.cancelled.store(true, Ordering::Relaxed);
        let (render, storage) = self
            .render_image_thread
            .join()
            .context("Unable to finish renderring thread");
        Ok((render, storage))
    }
}

pub fn run<R, D>(mut render: R, storage: D) -> Handle<R, D>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    D: DeviceStorage + Send + 'static,
    R::Error: std::fmt::Debug,
{
    // let cancelled = Arc::new(AtomicBool::new(false));

    // let is_cancelled = cancelled.clone();
    // let render_image_thread = std::thread::spawn(move || {
    //     {
    //         let config = storage.config().expect("Unable to read config");
    //         let image = storage
    //             .read_image(ImageId(config.current_image))
    //             .expect("Unable to read image");

    //         log::info!("Renderring {} image", config.current_image);

    //         let mut lines = ImageLines::new(image, config.strip_len);
    //         loop {
    //             if is_cancelled.load(Ordering::Relaxed) {
    //                 break;
    //             }

    //             let (line, frequency) = lines.next_line().expect("Unable to read next line");
    //             let duration = Duration::from_secs_f32(1.0 / frequency.0 as f32);
    //             log::info!("Showing next line");
    //             render
    //                 .write(line.into_iter())
    //                 .expect("Unable to show image line");
    //             std::thread::sleep(duration);
    //         }
    //     }

    //     (render, storage)
    // });

    // Handle {
    //     render_image_thread,
    //     cancelled,
    // }

    todo!()
}
