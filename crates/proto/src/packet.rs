use core::convert::TryInto;

pub use crate::types::FirmwareInfo;

use crate::types::{Hertz, MessageHeader};

pub const MAX_HEADER_LEN: usize = 80;

pub type PayloadLength = u32;

const PAYLOAD_LEN_BYTES: usize = core::mem::size_of::<PayloadLength>();

pub fn write_message_header(
    buf: &mut [u8],
    header: &MessageHeader,
    payload_len: usize,
) -> postcard::Result<usize> {
    let header_pos = PAYLOAD_LEN_BYTES + 1;

    let header_len = postcard::to_slice(header, &mut buf[header_pos..])?.len();
    assert!(header_len <= PayloadLength::MAX as usize);

    let packet_len: PayloadLength = payload_len.try_into().unwrap();

    buf[0] = header_len as u8;
    buf[1..header_pos].copy_from_slice(&packet_len.to_le_bytes());

    let total_len = header_len + PAYLOAD_LEN_BYTES + 1;
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

    pub fn read_message_len<I>(&mut self, bytes: &mut I) -> (usize, usize)
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(bytes.len() > (PAYLOAD_LEN_BYTES + 1));

        let header_len = bytes.next().unwrap() as usize;
        let payload_len = {
            let mut val_buf = [0_u8; PAYLOAD_LEN_BYTES];
            fill_buf(&mut val_buf, bytes, PAYLOAD_LEN_BYTES);

            PayloadLength::from_le_bytes(val_buf) as usize
        };

        (header_len, payload_len)
    }

    pub fn read_message<I>(
        &mut self,
        mut bytes: I,
        header_len: usize,
    ) -> postcard::Result<IncomingMessage<I>>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(self.header.len() >= header_len);
        fill_buf(&mut self.header, &mut bytes, header_len);

        let header: MessageHeader = postcard::from_bytes(&self.header)?;
        let msg = match header {
            MessageHeader::GetInfo => IncomingMessage::GetInfo,
            MessageHeader::Info(info) => IncomingMessage::Info(info),
            MessageHeader::Error(code) => IncomingMessage::Error(code),
            MessageHeader::ClearImages => IncomingMessage::ClearImages,

            MessageHeader::AddImage(img) => IncomingMessage::AddImage {
                refresh_rate: img.refresh_rate,
                bytes,
                strip_len: img.strip_len as usize,
            },
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
        refresh_rate: Hertz,
        strip_len: usize,
        bytes: I,
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
    assert!(buf.len() >= len);

    (0..len).for_each(|i| buf[i] = it.next().unwrap());
}
