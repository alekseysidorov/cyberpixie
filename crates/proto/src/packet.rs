pub use crate::types::FirmwareInfo;

use crate::types::MessageHeader;

pub const MAX_HEADER_LEN: usize = 80;

pub type PacketLength = u16;

const PACKET_LEN_BYTES: usize = core::mem::size_of::<PacketLength>();

pub fn write_message_header(buf: &mut [u8], header: &MessageHeader) -> postcard::Result<usize> {
    let used_len = postcard::to_slice(header, &mut buf[PACKET_LEN_BYTES..])?.len();
    assert!(used_len <= PacketLength::MAX as usize);

    let packet_len = used_len as PacketLength;
    buf[0..PACKET_LEN_BYTES].copy_from_slice(&packet_len.to_le_bytes());

    let total_len = used_len + PACKET_LEN_BYTES;
    Ok(total_len)
}

#[derive(Debug)]
pub struct PacketReader {
    header: [u8; MAX_HEADER_LEN],
}

impl Default for PacketReader {
    fn default() -> Self {
        Self {
            header: [0_u8; MAX_HEADER_LEN],
        }
    }
}

impl PacketReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_message_len<I>(&mut self, bytes: &mut I) -> usize
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(bytes.len() > PACKET_LEN_BYTES);

        let mut val_buf = [0_u8; PACKET_LEN_BYTES];
        fill_buf(&mut val_buf, bytes, PACKET_LEN_BYTES);

        PacketLength::from_le_bytes(val_buf) as usize
    }

    pub fn read_message<'a, I>(
        &mut self,
        bytes: &'a mut I,
        hdr_len: usize,
    ) -> postcard::Result<IncomingMessage<&'a mut I>>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(hdr_len <= self.header.len());
        fill_buf(&mut self.header, bytes, hdr_len);

        let header: MessageHeader = postcard::from_bytes(&self.header)?;
        let msg = match header {
            MessageHeader::GetInfo => IncomingMessage::GetInfo,
            MessageHeader::Info(info) => IncomingMessage::Info(info),
            MessageHeader::Error(code) => IncomingMessage::Error(code),
            MessageHeader::ClearImages => IncomingMessage::ClearImages,

            MessageHeader::AddImage(img) => {
                // assert_eq!(
                //     img.image_len as usize,
                //     bytes.len(),
                //     "The expected amount of bytes doesn't match the image size."
                // );

                IncomingMessage::AddImage {
                    refresh_rate: img.refresh_rate,
                    reader: bytes,
                    strip_len: img.strip_len as usize,
                }
            }
        };

        Ok(msg)
    }
}

#[derive(Debug)]
pub enum IncomingMessage<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    // Requests.
    GetInfo,
    AddImage {
        refresh_rate: u32,
        strip_len: usize,
        reader: I,
    },
    ClearImages,

    // Responses.
    Info(FirmwareInfo),
    Error(u16),
}

fn fill_buf<I>(buf: &mut [u8], it: &mut I, len: usize)
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    assert!(it.len() >= len);
    assert!(len <= buf.len());

    (0..len).for_each(|i| buf[i] = it.next().unwrap());
}
