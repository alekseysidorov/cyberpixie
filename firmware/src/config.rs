use cyberpixie::AppConfig;
use gd32vf103xx_hal::{
    serial::{Config, Parity, StopBits},
    time::{Bps, Hertz, MilliSeconds},
};

use crate::network::NetworkConfig;

pub const STRIP_LEDS_COUNT: usize = 48;

pub const SD_MMC_SPI_FREQUENCY: Hertz = Hertz(50_000_000);
pub const SD_MMC_SPI_TIMEOUT: u64 = 100_000;
pub const SOCKET_TIMEOUT: u64 = 60_000_000;
pub const WATCHDOG_DEADLINE: MilliSeconds = MilliSeconds(100);
pub const TIMER_TICK_FREQUENCY: Hertz = Hertz(20_000);

pub const SERIAL_PORT_CONFIG: Config = Config {
    baudrate: Bps(921600), // 460800, 921600
    parity: Parity::ParityNone,
    stopbits: StopBits::STOP1,
};

pub const ESP32_SERIAL_PORT_CONFIG: Config = Config {
    baudrate: Bps(115200),
    parity: Parity::ParityNone,
    stopbits: StopBits::STOP1,
};

pub const APP_CONFIG: AppConfig = AppConfig {
    current_image_index: 0,
    strip_len: STRIP_LEDS_COUNT as u16,
    safe_mode: false,
};

#[cfg(not(feature = "secondary_device"))]
pub const NETWORK_CONFIG: NetworkConfig<'static> = NetworkConfig::SoftAp {
    ssid: "cyberpixie",
    password: "12345678",
    channel: 5,
    mode: 0,
};
#[cfg(feature = "secondary_device")]
pub const NETWORK_CONFIG: NetworkConfig<'static> = NetworkConfig::JoinAp {
    ssid: "cyberpixie",
    password: "12345678",
    address: "192.168.4.1:333",
};
