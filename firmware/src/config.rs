use cyberpixie::AppConfig;
use esp8266_softap::{SoftApConfig, ADAPTER_BUF_CAPACITY};
use gd32vf103xx_hal::{
    serial::{Config, Parity, StopBits},
    time::Bps,
};

use crate::network::NetworkConfig;

pub const SERIAL_PORT_CONFIG: Config = Config {
    baudrate: Bps(921600), // Bps(460800),
    parity: Parity::ParityNone,
    stopbits: StopBits::STOP1,
};

pub const ESP32_SERIAL_PORT_CONFIG: Config = Config {
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

pub const APP_CONFIG: AppConfig = AppConfig {
    current_image_index: 0,
    strip_len: STRIP_LEDS_COUNT as u16,
    receiver_buf_capacity: ADAPTER_BUF_CAPACITY,
    safe_mode: true,
};

#[cfg(not(feature = "secondary_device"))]
pub const NETWORK_CONFIG: NetworkConfig<'static> = NetworkConfig::SoftAp {
    ssid: "cyberpixie",
    password: "12345678",
    channel: 5,
    mode: 4,
};
#[cfg(feature = "secondary_device")]
pub const NETWORK_CONFIG: NetworkConfig<'static> = NetworkConfig::JoinAp {
    ssid: "cyberpixie",
    password: "12345678",
    address: "192.168.4.1:333",
};
