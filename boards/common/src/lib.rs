//! Common Cyberpixie esp code.

#![no_std]
#![feature(async_fn_in_trait, type_alias_impl_trait)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation
)]

use esp_storage::FlashStorage;

pub mod render;

pub type StorageImpl = cyberpixie_embedded_storage::StorageImpl<FlashStorage>;

/// Creates a singleton value in the static memory and returns a mutable reference.
#[macro_export]
macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: static_cell::StaticCell<T> = static_cell::StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}
