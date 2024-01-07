#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self, clippy::missing_const_for_fn)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

pub use errors::{Error, Result};

pub mod errors;
pub mod io;
pub mod proto;

/// The maximum effective length of the pixel strip.
///
/// It doesn't make sense to create pixel devices with strip longer than this one,
/// the ws2812 protocol has not enough refresh rate.
pub const MAX_STRIP_LEN: usize = 48;
/// Bytes count per single pixel.
pub const BYTES_PER_PIXEL: usize = 3;
