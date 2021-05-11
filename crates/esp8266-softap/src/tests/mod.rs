use std::time::{Duration, Instant};

use cyberpixie_proto::{IncomingMessage, PacketReader};
use serial::EmbeddedSerial;

use crate::{
    adapter::Adapter,
    parser::CommandResponse,
    poll_continue,
    softap::{Event, SoftAp, SoftApConfig},
    BytesIter,
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
            Event::DataAvailable { link_id, mut reader } => {
                let mut packet_reader = PacketReader::default();
                let msg_len = packet_reader.read_message_len(&mut reader);
                let msg = packet_reader.read_message(reader, msg_len).unwrap();

                match msg {
                    IncomingMessage::GetInfo => {}
                    IncomingMessage::AddImage {
                        refresh_rate,
                        strip_len,
                        reader,
                        len,
                    } => {
                        for byte in BytesIter::new(link_id, reader, len) {
                            // eprint!("{} ", byte as char);
                            // buf.push(byte).unwrap();
                        }
                        eprintln!("Image read finished");

                        // let img_reader = RgbWriter::new(buf.as_slice().into_iter().copied());
                        // images.add_image(img_reader, refresh_rate.hz()).unwrap();
                        // uprintln!("Write image: total images count: {}", images.count());
                    }
                    IncomingMessage::ClearImages => {}
                    IncomingMessage::Info(_) => {}
                    IncomingMessage::Error(_) => {}
                };
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
