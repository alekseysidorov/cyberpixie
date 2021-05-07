use crate::{
    packet::write_message_header,
    types::{AddImage, FirmwareInfo, MessageHeader},
    IncomingMessage, PacketReader, MAX_HEADER_LEN,
};

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
    } = msg
    {
        assert_eq!(refresh_rate, 32);
        assert_eq!(strip_len, 24);
        assert_eq!(reader.len(), image_len);
        for byte in reader {
            assert_eq!(byte, 8);
        }
    } else {
        panic!("Wrong message type");
    }

    Ok(())
}
