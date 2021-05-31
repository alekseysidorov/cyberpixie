#![cfg_attr(not(test), no_std)]

#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(min_type_alias_impl_trait)]

pub use self::{
    app::{App, EventLoop},
    events::{HwEvent, HwEventSource},
    storage::{AppConfig, Storage},
};
pub use cyberpixie_proto as proto;
pub use smart_leds as leds;
pub use stdio_serial as stdio;

pub mod time;

mod app;
mod events;
mod storage;
