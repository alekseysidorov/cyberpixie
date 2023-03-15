#![cfg_attr(not(test), no_std)]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]
#![feature(min_type_alias_impl_trait)]

pub use cyberpixie_proto as proto;
pub use futures;
pub use nb_utils;
pub use no_stdout as stdout;
pub use smart_leds as leds;

pub use self::{
    app::App,
    events::HwEvent,
    storage::{AppConfig, Storage},
};

pub mod time;

mod app;
mod events;
mod storage;
