use crate::{AddImage, Message, PacketReader, Request, Response};

#[test]
fn postcard_messages() -> postcard::Result<()> {
    let mut buf = [8_u8; 512];

    let messages = [
        Message::Request(Request::AddImage(AddImage {
            refresh_rate: 50,
            len: 100,
        })),
        Message::Response(Response::Ok),
        Message::Bytes(b"Hello ebmedded world"),
    ];

    for message in &messages {
        let bytes = postcard::to_slice(&message, &mut buf)?;
        let message_2 = postcard::from_bytes(&bytes)?;
        assert_eq!(message, &message_2);
    }

    Ok(())
}

#[test]
fn packet_read_write() -> postcard::Result<()> {
    let mut buf = [0_u8; 512];

    let message = Message::Request(Request::AddImage(AddImage {
        refresh_rate: 50,
        len: 100,
    }));

    let packet = message.to_packet()?;
    let packet_len = packet.write_to(buf.as_mut());

    // Try to read the packet's length one byte at a time.
    let mut reader = PacketReader::new();
    reader.add_bytes(&buf[0..1]);
    reader.add_bytes(&buf[1..2]);
    let (packet_2, tail) = reader.add_bytes(&buf[2..]).unwrap();

    assert_eq!(&packet, packet_2);
    assert_eq!(tail.len(), buf.len() - packet_len);

    // Try to read the packet's length at the single read.
    reader.add_bytes(&buf[0..2]);
    let (packet_2, tail) = reader.add_bytes(&buf[2..]).unwrap();

    assert_eq!(&packet, packet_2);
    assert_eq!(tail.len(), buf.len() - packet_len);    

    // Try to read the whole packet by a single time.
    let (packet_2, tail) = reader.add_bytes(&buf).unwrap();
    assert_eq!(&packet, packet_2);
    assert_eq!(tail.len(), buf.len() - packet_len);

    Ok(())
}
