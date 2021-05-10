use core::fmt;

use embedded_hal::serial::Write;

pub fn with_writer<F>(_f: F) -> nb::Result<(), fmt::Error>
where
    F: FnOnce(&mut (dyn Write<u8, Error = fmt::Error> + 'static)) -> nb::Result<(), fmt::Error>,
{
    Ok(())
}

pub fn init<W>(_writer: W)
where
    W: Write<u8> + 'static + Send,
{
}
