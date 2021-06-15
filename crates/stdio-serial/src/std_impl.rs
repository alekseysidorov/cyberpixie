use std::fmt;

use embedded_hal::serial::Write;

struct StdoutWriter;

impl Write<u8> for StdoutWriter {
    type Error = fmt::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        print!("{}", word as char);
        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        Ok(())
    }
}

pub fn with_writer<F>(f: F) -> Result<(), fmt::Error>
where
    F: FnOnce(&mut (dyn Write<u8, Error = fmt::Error> + 'static)) -> Result<(), fmt::Error>,
{
    f(&mut StdoutWriter)
}

pub fn init<W>(_writer: W)
where
    W: Write<u8> + 'static + Send,
{
}
