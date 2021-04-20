#![no_std]

#![allow(incomplete_features)]

#![feature(min_type_alias_impl_trait)]
#![feature(generic_associated_types)]

pub use time::MicroSeconds;

pub mod stdout;
pub mod config;
pub mod strip;
pub mod generated;
pub mod time;
