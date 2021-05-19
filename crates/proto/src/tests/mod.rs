use std::{
    iter::Empty,
    time::{Duration, Instant},
};

use esp8266_softap::{Adapter, BytesIter, Event, SoftApConfig};

use crate::{
    packet::write_message_header,
    tests::serial::EmbeddedSerial,
    types::{AddImage, DeviceRole, FirmwareInfo, Hertz, MessageHeader},
    Error, Message, PacketReader, MAX_HEADER_LEN,
};

mod serial;

#[test]
fn postcard_messages() -> postcard::Result<()> {
    let mut buf = [8_u8; 512];

    let messages = [
        MessageHeader::Info(FirmwareInfo {
            version: [0, 0, 1, 0],
            device_id: [1, 2, 3, 4],
            role: DeviceRole::Host,
            strip_len: 24,
            images_count: 0,
        }),
        MessageHeader::Error(42),
        MessageHeader::AddImage(AddImage {
            refresh_rate: Hertz(50),
            strip_len: 24,
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
            version: [0, 0, 1, 0],
            device_id: [1, 2, 3, 4],
            role: DeviceRole::Host,
            strip_len: 24,
            images_count: 0,
        }),
        MessageHeader::Error(42),
        MessageHeader::GetInfo,
    ];

    let mut reader = PacketReader::new();
    for message in &messages {
        write_message_header(&mut buf, &message, 0)?;

        let mut bytes = buf.iter_mut().map(|x| *x);
        let (header_len, payload_len) = reader.read_message_len(&mut bytes);
        assert_eq!(payload_len, 0);
        assert!(header_len < bytes.len());

        let mut bytes = bytes.take(header_len);
        reader.read_message(&mut bytes, header_len)?;
    }

    Ok(())
}

#[test]
fn message_reader_unsized() -> postcard::Result<()> {
    let mut buf = [8_u8; 512];

    let image_len = 200;
    let message = MessageHeader::AddImage(AddImage {
        strip_len: 24,
        refresh_rate: Hertz(32),
    });
    let msg_len = write_message_header(&mut buf, &message, image_len)?;

    let mut reader = PacketReader::new();
    let mut bytes = buf.iter_mut().map(|x| *x).take(msg_len + image_len);
    let (header_len, payload_len) = reader.read_message_len(&mut bytes);

    let mut bytes = bytes.take(header_len + payload_len);
    let msg = reader.read_message(&mut bytes, header_len)?;
    if let Message::AddImage {
        refresh_rate,
        bytes,
        strip_len,
    } = msg
    {
        assert_eq!(refresh_rate, Hertz(32));
        assert_eq!(strip_len, 24);
        assert_eq!(bytes.len(), image_len);
        assert_eq!(bytes.len(), payload_len);

        for byte in bytes {
            assert_eq!(byte, 8);
        }
    } else {
        panic!("Wrong message type");
    }

    Ok(())
}

#[test]
fn message_into_bytes_scalar() {
    let messages: [Message<Empty<u8>>; 3] = [
        Message::Info(FirmwareInfo {
            version: [0, 0, 1, 0],
            device_id: [1, 2, 3, 4],
            role: DeviceRole::Host,
            strip_len: 24,
            images_count: 0,
        }),
        Message::Error(Error::ImageNotFound),
        Message::GetInfo,
    ];

    for message in messages {
        let mut bytes = message.into_bytes();

        let mut reader = PacketReader::default();
        let (header_len, _) = reader.read_message_len(&mut bytes);
        reader.read_message(&mut bytes, header_len).unwrap();
    }
}

#[test]
fn message_into_bytes_vector() {
    let buf = [42; 200];

    let msg = Message::AddImage {
        refresh_rate: Hertz(50),
        strip_len: 48,
        bytes: buf.iter().copied(),
    };

    let mut bytes = msg.into_bytes();
    let mut reader = PacketReader::default();
    let (header_len, _) = reader.read_message_len(&mut bytes);
    reader.read_message(&mut bytes, header_len).unwrap();
}

#[test]
#[ignore = "This test depends on the manual manipulations with the device."]
fn test_soft_ap() {
    let port = serialport::new("/dev/ttyUSB0", 115200).open().unwrap();
    let (rx, tx) = EmbeddedSerial::new(port).into_rx_tx();

    let mut ap = SoftApConfig {
        ssid: "cyberpixie",
        password: "12345678",
        channel: 5,
        mode: 4,
    }
    .start(Adapter::new(rx, tx).unwrap())
    .unwrap();

    eprintln!("Serial port established");
    let mut start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(15) {
            eprintln!("Timeouted");
            break;
        }

        let event = match ap.poll_next_event() {
            Ok(event) => event,
            Err(nb::Error::WouldBlock) => continue,
            Err(err) => panic!("{:?}", err),
        };

        match event {
            Event::Connected { link_id } => eprintln!("Event::Connected {}", link_id),
            Event::Closed { link_id } => {
                eprintln!("Event::Closed {}", link_id);
            }
            Event::DataAvailable {
                link_id,
                mut reader,
            } => {
                eprintln!("data available: {}", reader.len());

                let mut packet_reader = PacketReader::default();
                let (header_len, payload_len) = packet_reader.read_message_len(&mut reader);

                eprintln!("header_len: {}, payload_len: {}", header_len, payload_len);

                let bytes = BytesIter::new(link_id, reader, payload_len + header_len);
                let msg = packet_reader.read_message(bytes, header_len).unwrap();

                match msg {
                    Message::GetInfo => {}
                    Message::AddImage { bytes, .. } => {
                        let size = bytes.len();
                        for _byte in bytes {}

                        eprintln!("Image read finished, size: {}", size);

                        start = Instant::now();
                    }
                    Message::ClearImages => {}
                    Message::Info(_) => {}
                    Message::Error(_) => {}
                    Message::Ok => {}
                    Message::ImageAdded { .. } => {}
                    Message::ShowImage { .. } => {}
                };
            }
        }
    }
}
