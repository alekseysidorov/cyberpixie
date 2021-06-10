#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::{self, Write as _Write};

use embedded_hal::serial::Write;

#[cfg_attr(target_arch = "arm", path = "bare_metal_impl.rs")]
#[cfg_attr(target_arch = "riscv32", path = "bare_metal_impl.rs")]
#[cfg_attr(feature = "std", path = "std_impl.rs")]
#[cfg_attr(
    not(any(target_arch = "riscv32", target_arch = "arm", feature = "std")),
    path = "dummy_impl.rs"
)]
mod inner;

pub fn init<W>(writer: W)
where
    W: Write<u8> + 'static + Send,
{
    inner::init(writer)
}

/// Writes a single byte without blocking.
pub fn write_byte(word: u8) -> Result<(), fmt::Error> {
    inner::with_writer(|writer| nb::block!(writer.write(word)).map_err(|_| fmt::Error))
}

/// Writes a string to the configured serial port device.
pub fn write_str(s: &str) -> fmt::Result {
    inner::with_writer(|writer| {
        nb::block!(writer.write_str(s).map_err(nb::Error::Other))?;
        nb::block!(writer.flush()).map_err(|_| fmt::Error)
    })
}

/// Writes a formatted string to the configured serial port device.
pub fn write_fmt(args: fmt::Arguments) -> fmt::Result {
    inner::with_writer(|writer| {
        nb::block!(writer.write_fmt(args).map_err(nb::Error::Other))?;
        nb::block!(writer.flush()).map_err(|_| fmt::Error)
    })
}

/// Ensures that none of the previously written words are still buffered.
pub fn flush() -> fmt::Result {
    inner::with_writer(|writer| nb::block!(writer.flush()).map_err(|_| fmt::Error))
}

/// Macro for printing to the configured stdout, without a newline.
#[macro_export]
macro_rules! uprint {
    ($s:expr) => {{
        $crate::write_str($s).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::write_fmt(format_args!($s, $($tt)*)).ok();
    }};
}

/// Macro for printing to the configured stdout, with a newline.
#[macro_export]
macro_rules! uprintln {
    () => {{
        $crate::write_str(uprintln!(@newline)).ok();
    }};
    ($s:expr) => {{
        $crate::write_str(concat!($s, uprintln!(@newline))).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::write_fmt(format_args!(concat!($s, uprintln!(@newline)), $($tt)*)).ok();
    }};

    (@newline) => { "\r\n" };
}

/// Macro for printing to the configured stdout, without a newline.
///
/// This method prints only if the `dprint` feature enabled, which is useful
/// for debugging purposes.
#[cfg(any(feature = "dprint", doc))]
#[macro_export]
macro_rules! dprint {
    ($s:expr) => {{
        $crate::write_str($s).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::write_fmt(format_args!($s, $($tt)*)).ok();
    }};
}
#[cfg(not(any(feature = "dprint", doc)))]
#[macro_export]
macro_rules! dprint {
    ($s:expr) => {};
    ($s:expr, $($tt:tt)*) => {};
}

/// Macro for printing to the configured stdout, with a newline.
///
/// This method prints only if the `dprint` feature enabled, which is useful
/// for debugging purposes.
#[macro_export]
#[cfg(any(feature = "dprint", doc))]
macro_rules! dprintln {
    () => {{
        #[cfg(feature = "dprint")]
        $crate::write_str(uprintln!(@newline)).ok();
    }};
    ($s:expr) => {{
        #[cfg(feature = "dprint")]
        $crate::write_str(concat!($s, uprintln!(@newline))).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        #[cfg(feature = "dprint")]
        $crate::write_fmt(format_args!(concat!($s, uprintln!(@newline)), $($tt)*)).ok();
    }};

    (@newline) => { "\r\n" };
}
#[cfg(not(any(feature = "dprint", doc)))]
#[macro_export]
macro_rules! dprintln {
    () => {};
    ($s:expr) => {};
    ($s:expr, $($tt:tt)*) => {};
}
