use core::{
    cell::{RefCell, RefMut},
    fmt::{self, Write as FmtWrite},
};

use embedded_hal::serial::Write as SerialWrite;
use gd32vf103xx_hal::{pac::USART0, serial::Tx};
use no_stdout::StdOut;
use riscv::interrupt::{self, Mutex};

const STDOUT: StdOutImpl = StdOutImpl;
static USART0: Mutex<RefCell<Option<Tx<USART0>>>> = Mutex::new(RefCell::new(None));

pub fn init_stdout(tx: Tx<USART0>) {
    interrupt::free(|cs| {
        let mutex = USART0.borrow(*cs);
        mutex.borrow_mut().replace(tx);
    });

    no_stdout::init(&STDOUT).unwrap();
}

struct StdOutImpl;

impl StdOutImpl {
    fn with_usart0<F>(with_usart0: F) -> fmt::Result
    where
        F: Fn(RefMut<Tx<USART0>>) -> fmt::Result,
    {
        interrupt::free(|cs| {
            let mutex = USART0.borrow(*cs);
            let inner = RefMut::map(mutex.borrow_mut(), |o| o.as_mut().unwrap());
            with_usart0(inner)
        })
    }
}

impl StdOut for StdOutImpl {
    fn write_bytes(&self, bytes: &[u8]) -> core::fmt::Result {
        Self::with_usart0(|mut usart0| {
            for byte in bytes {
                nb::block!(usart0.write(*byte)).map_err(|_| fmt::Error)?;
            }
            Ok(())
        })
    }

    fn write_str(&self, s: &str) -> core::fmt::Result {
        Self::with_usart0(|mut usart0| usart0.write_str(s).map_err(|_| fmt::Error))
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        Self::with_usart0(|mut usart0| usart0.write_fmt(args).map_err(|_| fmt::Error))
    }

    fn flush(&self) -> core::fmt::Result {
        Self::with_usart0(|mut usart0| {
            // Unwrap is safe because the error type is Infallible.
            nb::block!(usart0.flush()).unwrap();
            Ok(())
        })
    }
}
