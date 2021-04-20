use core::fmt::{self, Write};

use embedded_hal::serial;

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

/// Writes a string to the specified serial port device.
pub fn write_str(tx: impl serial::Write<u8>, s: &str) -> fmt::Result {
    SerialWrapper(tx).write_str(s)
}

/// Writes a formatted string to the specified serial port device.
pub fn write_fmt(tx: impl serial::Write<u8>, args: fmt::Arguments) -> fmt::Result {
    SerialWrapper(tx).write_fmt(args)
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
