#![no_std]

#[cfg(not(feature = "without_alloc"))]
extern crate alloc;

use core::{cell::RefCell, fmt::{self, Write as _Write}, ops::DerefMut};

use embedded_hal::serial::Write;

#[cfg(not(feature = "without_alloc"))]
use alloc::boxed::Box;
#[cfg(feature = "without_alloc")]
use static_box::Box;

#[cfg(target_arch = "arm")]
use cortex_m::interrupt::{self, Mutex};
#[cfg(target_arch = "riscv32")]
use riscv::interrupt::{self, Mutex};

#[cfg(not(feature = "without_alloc"))]
type SerialWriter = Box<dyn Write<u8, Error = fmt::Error> + Send>;
#[cfg(feature = "without_alloc")]
type SerialWriter = Box<dyn Write<u8, Error = fmt::Error> + Send, 64>;

struct SerialWrapper<T: Write<u8>>(T);

impl<T: Write<u8>> Write<u8> for SerialWrapper<T> {
    type Error = fmt::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.0.write(word).map_err(|err| err.map(|_| fmt::Error))
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.0.flush().map_err(|err| err.map(|_| fmt::Error))
    }
}

fn box_serial_writer<W>(writer: W) -> SerialWriter
where
    W: Write<u8> + 'static + Send,
{
    Box::new(SerialWrapper(writer))
}

static STDOUT: Mutex<RefCell<Option<SerialWriter>>> = Mutex::new(RefCell::new(None));

fn with_writer<F>(f: F) -> nb::Result<(), fmt::Error>
where
    F: FnOnce(&mut (dyn Write<u8, Error = fmt::Error> + 'static)) -> nb::Result<(), fmt::Error>,
{
    interrupt::free(|cs| {
        let mut inner = STDOUT.borrow(cs).borrow_mut();
        if let Some(writer) = inner.as_mut() {
            f(writer.deref_mut())
        } else {
            Ok(())
        }
    })
}

pub fn init<W>(writer: W)
where
    W: Write<u8> + 'static + Send,
{
    let boxed = box_serial_writer(writer);
    interrupt::free(|cs| {
        let mut inner = STDOUT.borrow(cs).borrow_mut();
        inner.replace(boxed);
    })
}

/// Writes a single byte without blocking.
pub fn write_byte(word: u8) -> nb::Result<(), fmt::Error> {
    with_writer(|writer| writer.write(word))
}

/// Writes a string to the configured serial port device.
pub fn write_str(s: &str) -> fmt::Result {
    nb::block!(with_writer(|writer| writer
        .write_str(s)
        .map_err(nb::Error::Other)))
}

/// Writes a formatted string to the configured serial port device.
pub fn write_fmt(args: fmt::Arguments) -> fmt::Result {
    nb::block!(with_writer(|writer| writer
        .write_fmt(args)
        .map_err(nb::Error::Other)))
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
