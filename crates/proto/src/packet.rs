pub use crate::types::FirmwareInfo;

use core::{convert::TryInto, iter::Empty, mem::MaybeUninit};

use crate::types::{AddImage, Hertz, MessageHeader};

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

    let total_header_len = header_len + PAYLOAD_LEN_BYTES + 1;
    Ok(total_header_len)
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
    pub const PACKET_LEN_BUF_SIZE: usize = PAYLOAD_LEN_BYTES + 1;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_message_len<I>(&self, bytes: &mut I) -> (usize, usize)
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(bytes.len() >= Self::PACKET_LEN_BUF_SIZE);

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
    ) -> postcard::Result<Message<I>>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(self.header.len() >= header_len);
        fill_buf(&mut self.header, &mut bytes, header_len);

        let header: MessageHeader = postcard::from_bytes(&self.header)?;
        let msg = match header {
            MessageHeader::GetInfo => Message::GetInfo,
            MessageHeader::Info(info) => Message::Info(info),
            MessageHeader::Error(code) => Message::Error(code),
            MessageHeader::ClearImages => Message::ClearImages,
            MessageHeader::AddImage(img) => Message::AddImage {
                refresh_rate: img.refresh_rate,
                bytes,
                strip_len: img.strip_len as usize,
            },
            MessageHeader::Ok => Message::Ok,
            MessageHeader::ImageAdded(index) => Message::ImageAdded {
                index: index as usize,
            },
        };

        Ok(msg)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Message<I = Empty<u8>>
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
    Ok,
    ImageAdded {
        index: usize,
    },
    Info(FirmwareInfo),
    Error(u16),
}

impl<I> Message<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    fn into_header_payload(self) -> (MessageHeader, Option<I>) {
        match self {
            Message::GetInfo => (MessageHeader::GetInfo, None),
            Message::AddImage {
                refresh_rate,
                strip_len,
                bytes,
            } => (
                MessageHeader::AddImage(AddImage {
                    refresh_rate,
                    strip_len: strip_len as u16,
                }),
                Some(bytes),
            ),
            Message::ClearImages => (MessageHeader::ClearImages, None),
            Message::Ok => (MessageHeader::Ok, None),
            Message::ImageAdded { index } => (MessageHeader::ImageAdded(index as u16), None),
            Message::Info(info) => (MessageHeader::Info(info), None),
            Message::Error(code) => (MessageHeader::Error(code), None),
        }
    }

    pub fn into_bytes(self) -> MessageBytes<I> {
        let mut header_buf = {
            let uninit: MaybeUninit<[u8; MAX_HEADER_LEN]> = MaybeUninit::uninit();
            // Safety: We know how many bytes will be used and primitive types
            // don't have drop implementation.
            unsafe { uninit.assume_init() }
        };

        let (header, payload) = self.into_header_payload();
        let payload_len = payload
            .as_ref()
            .map(|payload| payload.len())
            .unwrap_or_default();

        let header_len = write_message_header(&mut header_buf, &header, payload_len)
            .expect("Unable to serialize message");

        MessageBytes::new(header_buf, header_len, payload)
    }
}

impl Message<Empty<u8>> {
    pub fn get_info() -> Self {
        Self::GetInfo
    }

    pub fn image_added(index: usize) -> Self {
        Self::ImageAdded { index }
    }

    pub fn clear_images() -> Self {
        Self::ClearImages
    }
}

pub struct MessageBytes<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    buf: [u8; MAX_HEADER_LEN],
    len: usize,
    current_byte: usize,
    payload: Option<I>,
}

impl<I> MessageBytes<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    fn new(buf: [u8; MAX_HEADER_LEN], len: usize, payload: Option<I>) -> Self {
        Self {
            buf,
            len,
            current_byte: 0,
            payload,
        }
    }
}

impl<I> Iterator for MessageBytes<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_byte < self.len {
            let byte = self.buf[self.current_byte];
            self.current_byte += 1;
            return Some(byte);
        }

        if let Some(payload) = self.payload.as_mut() {
            payload.next()
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut len = self.len - self.current_byte;
        if let Some(payload) = self.payload.as_ref() {
            len += payload.len();
        }

        (len, Some(len))
    }
}

impl<I> ExactSizeIterator for MessageBytes<I> where I: Iterator<Item = u8> + ExactSizeIterator {}

fn fill_buf<I>(buf: &mut [u8], it: &mut I, len: usize)
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    assert!(it.len() >= len);
    assert!(buf.len() >= len);

    (0..len).for_each(|i| buf[i] = it.next().unwrap());
}
