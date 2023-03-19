use std::{net::TcpListener, time::Duration};

use cyberpixie_core::{
    proto::types::{DeviceInfo, DeviceRole},
    service::DeviceService,
};
use cyberpixie_esp32c3::{
    storage::ImagesRegistry,
    wifi::{Config, Wifi},
};
use cyberpixie_std_network::NetworkPart;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger};
use esp_idf_sys as _;
use log::info; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sysloop = EspSystemEventLoop::take()?;
    let mut wifi = Wifi::new(peripherals.modem, sysloop)?;
    wifi.establish_softap(Config::default())?;

}
