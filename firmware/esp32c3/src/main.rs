use std::net::TcpListener;

use cyberpixie_esp32c3::wifi::{Config, Wifi};
use cyberpixie_proto::{nb, DeviceRole, FirmwareInfo, Handshake, Message, Service, SimpleMessage};
use cyberpixie_std_transport::{display_err, TcpTransport};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger};
use esp_idf_sys as _;
use log::info; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

const DEVICE_ID: [u32; 4] = [6, 6, 6, 6];

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

    let stream = listener.incoming().next().unwrap()?;
    let address = stream.local_addr()?;

    let mut service = Service::new(TcpTransport::new(address, stream), 640);
    info!("Created Service");
    loop {
        info!("Waiting for next message...");
        let (address, message) = nb::block!(service.poll_next_message())?;
        let response = match message {
            cyberpixie_proto::Message::HandshakeRequest(handshake) => {
                info!("Got handshake: {:?}", handshake);
                Some(SimpleMessage::HandshakeResponse(Handshake {
                    device_id: DEVICE_ID,
                    group_id: None,
                    role: DeviceRole::Main,
                }))
            }
            cyberpixie_proto::Message::GetInfo => Some(Message::Info(FirmwareInfo {
                strip_len: 36,
                version: [0, 0, 0, 0],
                images_count: 0,
                device_id: DEVICE_ID,
                role: DeviceRole::Main,
            })),

            other => {
                info!("Got an unsupported message: {:?}", other);

                None
            }
        };
        service.confirm_message(address)?;
        
        if let Some(response) = response {
            service.send_message(address, response)?;
        }
    }
}
