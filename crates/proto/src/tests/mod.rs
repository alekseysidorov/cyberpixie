use std::time::{Duration, Instant};

use esp8266_softap::{Adapter, BytesIter, Event, SoftAp, SoftApConfig};

use crate::{
    packet::write_message_header,
    tests::serial::EmbeddedSerial,
    types::{AddImage, FirmwareInfo, MessageHeader},
    IncomingMessage, PacketReader, MAX_HEADER_LEN,
};

mod serial;

#[macro_export]
macro_rules! poll_continue {
    ($e:expr) => {
        match $e {
            Err(nb::Error::WouldBlock) => continue,
            other => other,
        }
    };
}

#[test]
fn postcard_messages() -> postcard::Result<()> {
    let mut buf = [8_u8; 512];

    let messages = [
        MessageHeader::Info(FirmwareInfo {
            version: 1,
            strip_len: 24,
        }),
        MessageHeader::Error(42),
        MessageHeader::AddImage(AddImage {
            refresh_rate: 32,
            strip_len: 24,
            image_len: 15,
        }),
    ];

    for message in &messages {
        let bytes = postcard::to_slice(&message, &mut buf)?;
        let message_2 = postcard::from_bytes(&bytes)?;
        assert_eq!(message, &message_2);
    }

    Ok(())
}

#[test]
fn message_reader_scalar() -> postcard::Result<()> {
    let mut buf = [8_u8; MAX_HEADER_LEN];

    let messages = [
        MessageHeader::Info(FirmwareInfo {
            version: 1,
            strip_len: 24,
        }),
        MessageHeader::Error(42),
        MessageHeader::GetInfo,
    ];

    let mut reader = PacketReader::new();
    for message in &messages {
        write_message_header(&mut buf, &message)?;

        let mut bytes = buf.iter_mut().map(|x| *x);
        let len = reader.read_message_len(&mut bytes);

        assert!(len < bytes.len());
        let mut bytes = bytes.take(len);
        reader.read_message(&mut bytes, len)?;
    }

    Ok(())
}

#[test]
fn message_reader_unsized() -> postcard::Result<()> {
    let mut buf = [8_u8; 512];

    let image_len = 200;
    let message = MessageHeader::AddImage(AddImage {
        image_len: image_len as u32,
        strip_len: 24,
        refresh_rate: 32,
    });
    write_message_header(&mut buf, &message)?;

    let mut reader = PacketReader::new();
    let mut bytes = buf.iter_mut().map(|x| *x);
    let len = reader.read_message_len(&mut bytes);

    let mut bytes = bytes.take(len + image_len);
    let msg = reader.read_message(&mut bytes, len)?;
    if let IncomingMessage::AddImage {
        refresh_rate,
        reader,
        strip_len,
        len,
    } = msg
    {
        assert_eq!(refresh_rate, 32);
        assert_eq!(strip_len, 24);
        assert_eq!(reader.len(), image_len);
        assert_eq!(len, image_len);

        for byte in reader {
            assert_eq!(byte, 8);
        }
    } else {
        panic!("Wrong message type");
    }

    Ok(())
}

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
            Event::DataAvailable {
                link_id,
                mut reader,
            } => {
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
