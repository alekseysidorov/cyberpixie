use cyberpixie::AppConfig;
use esp8266_softap::{SoftApConfig, ADAPTER_BUF_CAPACITY};
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

pub const DEFAULT_APP_CONFIG: AppConfig = AppConfig {
    current_image_index: 0,
    strip_len: STRIP_LEDS_COUNT as u16,
    receiver_buf_capacity: ADAPTER_BUF_CAPACITY,
    safe_mode: true,
};
