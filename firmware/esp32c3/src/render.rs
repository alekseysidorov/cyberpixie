//!

use std::{
    sync::mpsc::Receiver,
    time::{Duration, SystemTime},
};

use cyberpixie_core::service::{DeviceService, DeviceStorage, ImageLines};
use smart_leds::{SmartLedsWrite, RGB8};

fn start_rendering_thread<R, I>(mut render: R, frames: Receiver<(I, Duration)>) -> !
where
    R: SmartLedsWrite<Color = RGB8>,
    I: Iterator<Item = RGB8>,
    R::Error: std::fmt::Debug,
{
    loop {
        let (frame, duration) = frames.recv().expect("Unable to recieve next image frame");
        render.write(frame).expect("Unable to show image line");
        std::thread::sleep(duration);
    }
}

pub struct Handle {
    // read_image_thread: 
}

pub fn render_image<R>(mut render: R, storage: D)
where
    R: SmartLedsWrite<Color = RGB8>,
    D: DeviceStorage,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(24);

    let read_image_thread = std::thread::spawn(move || {
        let config = storage.config().expect("Unable to read config");
        let image = storage.read_image(config.current_image).expect("Unable to read image");
        let lines = ImageLines::new(image, config.strip_len);

        loop {
            let (line, frequency) = lines.next_line().expect("Unable to read next image line");

            tx.send(line).expect("Unable to send line to the rendering thread");
        }
    });


    todo!()
}
