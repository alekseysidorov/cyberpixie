#[cfg(not(feature = "without_alloc"))]
extern crate alloc;

use core::{cell::RefCell, fmt, ops::DerefMut};

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
type SerialWriter = Box<dyn Write<u8, Error = fmt::Error> + Send, [u8; 32]>;

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

pub fn with_writer<F>(f: F) -> nb::Result<(), fmt::Error>
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
