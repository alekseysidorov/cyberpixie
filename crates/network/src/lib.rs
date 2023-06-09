//! Cyberpixie Network abstraction layer
//!
//! This crate provides a implementation agnostic network layer for the cyberpixie project.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Features
#![feature(async_fn_in_trait)]
// Linter configuration
#![warn(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::missing_const_for_fn
)]

pub use cyberpixie_core as core;
use cyberpixie_core::{Error as CyberpixieError, Result as CyberpixieResult};
pub use embedded_nal::SocketAddr;

pub use crate::message::{Message, PayloadReader};

pub mod asynch;
pub mod blocking;
pub mod message;
