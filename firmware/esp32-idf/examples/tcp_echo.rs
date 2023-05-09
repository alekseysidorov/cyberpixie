use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

use cyberpixie_esp32_idf::wifi::{Config, Wifi};
use esp_idf_hal::prelude::*;
use esp_idf_svc::{eventloop::EspSystemEventLoop, log::EspLogger};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;
use log::info;

fn run_echo_server(mut stream: TcpStream) -> anyhow::Result<()> {
    // read 20 bytes at a time from stream echoing back to stream
    loop {
        let mut read = [0; 1028];
        let n = stream.read(&mut read)?;
        if n == 0 {
            // connection was closed
            break;
        }

        let bytes = &read[0..n];
        log::info!("-> {}", std::str::from_utf8(bytes)?);
        stream.write_all(bytes)?;

        if bytes.starts_with(b"exit") {
            break;
        }
    }

    Ok(())
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

    for stream in listener.incoming() {
        let stream = stream?;
        info!("Got connection: {:?}", stream.peer_addr());
        std::thread::spawn(move || run_echo_server(stream));
    }
    Ok(())
}
