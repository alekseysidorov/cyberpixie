use embedded_hal::serial::{Read, Write};
use serial::EmbeddedSerial;

use crate::{adapter::Adapter, parser::NetworkEvent, softap::{SoftAp, SoftApConfig}};

mod serial;

#[test]
fn test_soft_ap() {
    let port = serialport::new("/dev/ttyUSB0", 115200).open().unwrap();
    let (rx, tx) = EmbeddedSerial::new(port).into_rx_tx();

    let adapter = Adapter::new(rx, tx).unwrap();
    let (mut rx, tx) = SoftAp::new(adapter)
        .start(SoftApConfig {
            ssid: "aurora_led",
            password: "12345678",
            channel: 5,
            mode: 4,
        })
        .unwrap();

    loop {
        let event = nb::block!(rx.poll_data()).unwrap();
        eprintln!("Got event: {:?}", event);
    }
}

#[test]
fn test_parse_connect() {
    let raw = b"1,CONNECT\r\n";
    let event = NetworkEvent::parse(raw.as_ref()).unwrap().1;
    
    assert_eq!(event, NetworkEvent::Connected { link_id: 1 })
}

#[test]
fn test_parse_close() {
    let raw = b"1,CLOSED\r\n";
    let event = NetworkEvent::parse(raw.as_ref()).unwrap().1;
    
    assert_eq!(event, NetworkEvent::Closed { link_id: 1 })
}

#[test]
fn test_parse_data_available() {
    let raw = b"+IPD,12,6:hello\r\n";
    let event = NetworkEvent::parse(raw.as_ref()).unwrap().1;
    
    assert_eq!(event, NetworkEvent::DataAvailable { link_id: 12, size: 6 })
}
