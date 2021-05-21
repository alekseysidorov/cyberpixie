use crate::types::{AddImage, DeviceRole, FirmwareInfo, Hertz, MessageHeader};

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
            bytes_len: 12,
        }),
    ];

    for message in &messages {
        let bytes = postcard::to_slice(&message, &mut buf)?;
        let message_2 = postcard::from_bytes(&bytes)?;
        assert_eq!(message, &message_2);
    }

    Ok(())
}
