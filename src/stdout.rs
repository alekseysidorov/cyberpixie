use core::fmt::{self, Write};

use embedded_hal::serial;
use gd32vf103xx_hal::{pac::USART0, serial::Tx};

use crate::sync::RwLock;

/// Wraps the original serial writer to handle `\ n` symbols as` \ r` for better
/// compatibility with the some of the devices.
struct SerialWrapper<S>(S);

impl<S> Write for SerialWrapper<S>
where
    S: serial::Write<u8>,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.as_bytes() {
            if *byte == b'\n' {
                nb::block!(self.0.write(b'\r')).map_err(|_| fmt::Error)?;
            }

            nb::block!(self.0.write(*byte)).map_err(|_| fmt::Error)?;
        }

        Ok(())
    }
}

static STDOUT: RwLock<Option<Tx<USART0>>> = RwLock::new(None);

pub fn enable(tx: Tx<USART0>) {
    STDOUT
        .write(|mut inner| {
            inner.replace(tx);
        })
        .unwrap();
}

pub fn release() -> Option<Tx<USART0>> {
    STDOUT.write(|mut inner| inner.take()).unwrap()
}

/// Writes a string to the configured serial port device.
pub fn write_str(s: &str) -> fmt::Result {
    STDOUT
        .write(|mut inner| {
            if let Some(tx) = inner.as_mut() {
                tx.write_str(s)
            } else {
                Ok(())
            }
        })
        .unwrap()
}

/// Writes a formatted string to the configured serial port device.
pub fn write_fmt(args: fmt::Arguments) -> fmt::Result {
    STDOUT
        .write(|mut inner| {
            if let Some(tx) = inner.as_mut() {
                tx.write_fmt(args)
            } else {
                Ok(())
            }
        })
        .unwrap()
}

/// Macro for printing to the specified output, without a newline.
#[macro_export]
macro_rules! uwrite {
    ($o:expr, $s:expr) => {{
        use core::fmt::Write;
        $o.write_str($s).ok();
    }};
    ($o:expr, $s:expr, $($tt:tt)*) => {{
        use core::fmt::Write;
        $o.write_fmt(format_args!($s, $($tt)*)).ok();
    }};
}

/// Macro for printing to the specified output, with a newline.
#[macro_export]
macro_rules! uwriteln {
    ($o:expr) => {{
        use core::fmt::Write;
        $o.write_str("\r\n").ok();
    }};
    ($o:expr, $s:expr) => {{
        use core::fmt::Write;
        $o.write_str(concat!($s, "\r\n")).ok();
    }};
    ($o:expr, $s:expr, $($tt:tt)*) => {{
        use core::fmt::Write;
        $o.write_fmt(format_args!(concat!($s, "\r\n"), $($tt)*)).ok();
    }};
}

/// Macro for printing to the configured stdout, without a newline.
#[macro_export]
macro_rules! uprint {
    ($s:expr) => {{
        $crate::stdout::write_str($s).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::stdout::write_fmt(format_args!($s, $($tt)*)).ok();
    }};
}

/// Macro for printing to the configured stdout, without a newline.
#[macro_export]
macro_rules! uprintln {
    () => {{
        $crate::stdout::write_str("\r\n").ok();
    }};
    ($s:expr) => {{
        $crate::stdout::write_str(concat!($s, "\r\n")).ok();
    }};
    ($s:expr, $($tt:tt)*) => {{
        $crate::stdout::write_fmt(format_args!(concat!($s, "\r\n"), $($tt)*)).ok();
    }};
}
