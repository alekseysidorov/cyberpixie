use std::{net::TcpListener, time::Duration};

use cyberpixie_core::{
    proto::types::{DeviceInfo, DeviceRole},
    service::DeviceService,
};
use cyberpixie_esp32c3::{
    storage::ImagesRegistry,
    wifi::{Config, Wifi},
    DEFAULT_DEVICE_CONFIG,
};
use cyberpixie_std_network::NetworkPart;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger};
use esp_idf_sys as _;
use log::info; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

struct DeviceStub;

impl DeviceService for DeviceStub {
    type Storage = ImagesRegistry;
    type ImageRender = ();

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            role: DeviceRole::Main,
            group_id: None,
            strip_len: Some(DEFAULT_DEVICE_CONFIG.strip_len),
        }
    }

    fn storage(&self) -> Self::Storage {
        ImagesRegistry::new(DEFAULT_DEVICE_CONFIG)
    }

    fn show_current_image(&mut self) -> Self::ImageRender {
        todo!()
    }
}

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sysloop = EspSystemEventLoop::take()?;
    let mut wifi = Wifi::new(peripherals.modem, sysloop)?;
    wifi.establish_softap(Config::default())?;

    let listener = TcpListener::bind("0.0.0.0:80")?;
    info!("Bound TCP on: {:?}", listener.local_addr());

    let mut server = NetworkPart::new(DeviceStub, listener)?;
    info!("Created Service");
    loop {
        if let Err(nb::Error::Other(err)) = server.poll() {
            panic!("{err}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
