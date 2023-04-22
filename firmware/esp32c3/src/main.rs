use std::{net::TcpListener, time::Duration};

use cyberpixie_core::proto::types::Hertz;
use cyberpixie_esp32c3::{
    render::Render,
    splash::WanderingLight,
    storage::ImagesRegistry,
    wifi::{Config, Wifi},
    DeviceImpl, DEFAULT_DEVICE_CONFIG, LED_PIN, STRIP_LEN,
};
use cyberpixie_std_network::NetworkPart;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use log::info;

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

    // Initialize and clear strip.
    let mut strip = Render::new(0, LED_PIN)?;
    strip.clear(144)?;
    // Show splash
    let splash = WanderingLight::<STRIP_LEN>::new(64);
    let rate = Hertz(100);
    for (_ticks, line) in splash {
        strip.write(line.into_iter().collect())?;
        std::thread::sleep(Duration::from(rate));
    }

    // Initialize device.
    let storage = ImagesRegistry::new(DEFAULT_DEVICE_CONFIG);
    let device = DeviceImpl::new(storage, strip)?;

    // Start server.
    let mut server = NetworkPart::new(device, listener)?;
    info!("Created Service");
    loop {
        if let Err(nb::Error::Other(err)) = server.poll() {
            panic!("{err}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
