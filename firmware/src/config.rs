use esp8266_softap::SoftApConfig;
use gd32vf103xx_hal::{
    serial::{Config, Parity, StopBits},
    time::Bps,
};

pub const SERIAL_PORT_CONFIG: Config = Config {
    baudrate: Bps(115200),
    parity: Parity::ParityNone,
    stopbits: StopBits::STOP1,
};

pub const SOFTAP_CONFIG: SoftApConfig = SoftApConfig {
    ssid: "cyberpixie",
    password: "12345678",
    channel: 5,
    mode: 4,
};

pub const STRIP_LEDS_COUNT: usize = 48;
pub const MAX_LINES_COUNT: usize = 180;
pub const MAX_IMAGE_BUF_SIZE: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;
