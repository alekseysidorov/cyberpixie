use gd32vf103xx_hal::{
    serial::{Config, Parity, StopBits},
    time::Bps,
};

pub const SERIAL_PORT_CONFIG: Config = Config {
    baudrate: Bps(115200),
    parity: Parity::ParityNone,
    stopbits: StopBits::STOP1,
};

pub const STRIP_LEDS_COUNT: usize = 24;

pub const MAX_LINES_COUNT: usize = 150;
