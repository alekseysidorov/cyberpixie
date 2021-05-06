#![no_std]

// #![allow(incomplete_features)]
// #![feature(min_type_alias_impl_trait)]
// #![feature(generic_associated_types)]
// #![feature(const_generics)]
// #![feature(const_evaluatable_checked)]

pub use stdio_serial::{uprint, uprintln};
pub use time::Microseconds;

pub mod allocator;
pub mod config;
#[cfg(feature = "generate_img")]
pub mod generated;
pub mod stdio;
pub mod storage;
pub mod strip;
pub mod sync;
pub mod time;
