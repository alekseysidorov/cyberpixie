use core::{
    cell::{RefCell, RefMut},
    fmt,
};

use embedded_hal::serial::Write;
use gd32vf103xx_hal::{pac::USART0, serial::Tx};
use log::Log;
use riscv::interrupt::{self, Mutex};

struct LoggerContext {
    tx: Tx<USART0>,
    log_level: log::Level,
}

static LOGGER_CONTEXT: Mutex<RefCell<Option<LoggerContext>>> = Mutex::new(RefCell::new(None));

impl LoggerContext {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.log_level >= metadata.level()
    }

    fn as_fmt_write(&mut self) -> &mut dyn fmt::Write {
        &mut self.tx
    }
}

struct SerialLogger;

impl SerialLogger {
    fn access_context<R, F: FnOnce(RefMut<LoggerContext>) -> R>(&self, f: F) -> R {
        interrupt::free(|cs| {
            let context = RefMut::map(LOGGER_CONTEXT.borrow(cs).borrow_mut(), |x| {
                x.as_mut().expect("logger context should be inited")
            });
            f(context)
        })
    }
}

impl Log for SerialLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.access_context(|ctx| ctx.enabled(metadata))
    }

    fn log(&self, record: &log::Record) {
        self.access_context(|mut ctx| {
            if ctx.enabled(record.metadata()) {
                ctx.as_fmt_write()
                    .write_fmt(format_args!("[{}] - {}", record.level(), record.args()))
                    .ok();
            }
        });
    }

    fn flush(&self) {
        self.access_context(|mut ctx| {
            ctx.tx.flush().ok();
        })
    }
}

pub fn init_logger(tx: Tx<USART0>, log_level: log::Level) {
    interrupt::free(|cs| {
        LOGGER_CONTEXT
            .borrow(cs)
            .replace(Some(LoggerContext { tx, log_level }))
    });

    static SERIAL_LOGGER: SerialLogger = SerialLogger;
    log::set_logger(&SERIAL_LOGGER).unwrap()
}
