#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use self::{
    app::{App, AppConfig},
    images::ImagesRepository,
};
pub use cyberpixie_proto as proto;
pub use smart_leds as leds;
pub use stdio_serial as stdio;

pub mod time;

mod app;
mod images;
