use std::fmt;

use embedded_hal::serial;
use serialport::SerialPort;

pub struct EmbeddedSerial(Box<dyn SerialPort>);

impl EmbeddedSerial {
    pub fn new(inner: Box<dyn SerialPort>) -> Self {
        Self(inner)
    }

    pub fn into_rx_tx(self) -> (impl serial::Read<u8>, impl serial::Write<u8>) {
        let rx = self.0.try_clone().unwrap();

        (Self(rx), self)
    }
}

impl serial::Read<u8> for EmbeddedSerial {
    type Error = std::io::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buf = [0_u8];
        match self.0.read(&mut buf) {
            Ok(_) => {
                // eprint!("{}", buf[0] as char);
                Ok(buf[0])
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::Interrupted =>
            {
                Err(nb::Error::WouldBlock)
            }
            Err(e) => Err(nb::Error::Other(e)),
        }
    }
}

impl serial::Write<u8> for EmbeddedSerial {
    type Error = std::io::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        let buf = [word];
        match self.0.write(&buf) {
            Ok(_) => {
                // eprint!("{}", word as char);
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Err(nb::Error::WouldBlock),
            Err(e) => Err(nb::Error::Other(std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            ))),
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.0
            .clear(serialport::ClearBuffer::All)
            .map_err(|e| nb::Error::Other(std::io::Error::new(std::io::ErrorKind::Other, e)))
    }
}

impl fmt::Write for EmbeddedSerial {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        (self as &mut dyn serial::Write<u8, Error = std::io::Error>).write_str(s)
    }
}
