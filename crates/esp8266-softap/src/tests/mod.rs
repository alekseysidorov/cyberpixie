use std::time::{Duration, Instant};

use serial::EmbeddedSerial;

use crate::{
    adapter::Adapter,
    parser::CommandResponse,
    poll_continue,
    softap::{Event, SoftAp, SoftApConfig},
};

mod serial;

#[test]
fn test_soft_ap() {
    let port = serialport::new("/dev/ttyUSB0", 115200).open().unwrap();
    let (rx, tx) = EmbeddedSerial::new(port).into_rx_tx();

    let adapter = Adapter::new(rx, tx).unwrap();
    let (mut rx, _tx) = SoftAp::new(adapter)
        .start(SoftApConfig {
            ssid: "cyberpixie",
            password: "12345678",
            channel: 5,
            mode: 4,
        })
        .unwrap();

    eprintln!("Serial port established");
    let start = Instant::now();
    loop {
        if start.elapsed() == Duration::from_secs(60) {
            eprintln!("Ignored due timeout");
            break;
        }

        let event = poll_continue!(rx.poll_data()).unwrap();
        match event {
            Event::Connected { link_id } => eprintln!("Event::Connected {}", link_id),
            Event::Closed { link_id } => {
                eprintln!("Event::Closed {}", link_id);
                break;
            }
            Event::DataAvailable { link_id, reader } => {
                eprintln!("Event::BytesReceived {} count: {}", link_id, reader.len());
                for byte in reader {
                    eprint!("{}", byte as char);
                }
                eprintln!();
            }
        }
    }
}

#[test]
fn test_parse_connect() {
    let raw = b"1,CONNECT\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(event, CommandResponse::Connected { link_id: 1 })
}

#[test]
fn test_parse_close() {
    let raw = b"1,CLOSED\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(event, CommandResponse::Closed { link_id: 1 })
}

#[test]
fn test_parse_data_available() {
    let raw = b"+IPD,12,6:hello\r\n";
    let event = CommandResponse::parse(raw.as_ref()).unwrap().1;

    assert_eq!(
        event,
        CommandResponse::DataAvailable {
            link_id: 12,
            size: 6
        }
    )
}
