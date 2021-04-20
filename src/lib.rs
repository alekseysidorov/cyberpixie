#![no_std]
#![allow(incomplete_features)]
#![feature(min_type_alias_impl_trait)]
#![feature(generic_associated_types)]

pub use time::Microseconds;

pub mod config;
pub mod generated;
pub mod stdout;
pub mod strip;
pub mod time;
