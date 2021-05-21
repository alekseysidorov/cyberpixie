pub use crate::types::FirmwareInfo;

use core::{convert::TryInto, iter::Empty, mem::MaybeUninit};

use displaydoc::Display;

use crate::types::{AddImage, Hertz, MessageHeader};

pub const MAX_HEADER_LEN: usize = 80;

type PayloadLength = u32;
type HeaderLength = u16;

const PAYLOAD_LEN_BYTES: usize = core::mem::size_of::<PayloadLength>();
const HEADER_LEN_BYTES: usize = core::mem::size_of::<HeaderLength>();

macro_rules! read_le {
    ($type:tt, &mut $bytes:expr) => {{
        let mut val_buf = [0_u8; core::mem::size_of::<$type>()];
        fill_buf(&mut val_buf, $bytes, core::mem::size_of::<$type>());
        $type::from_le_bytes(val_buf)
    }};
}

macro_rules! write_le {
    ($type:tt, $value:expr, $bytes:expr) => {{
        let value: $type = $value.try_into().unwrap();
        let len = core::mem::size_of::<$type>();
        $bytes[0..len].copy_from_slice(&value.to_le_bytes());
        &mut $bytes[len..]
    }};
}

pub fn write_message_header(
    mut buf: &mut [u8],
    header: &MessageHeader,
    payload_len: usize,
) -> postcard::Result<usize> {
    let header_pos = PAYLOAD_LEN_BYTES + HEADER_LEN_BYTES;

    let header_len = postcard::to_slice(header, &mut buf[header_pos..])?.len();
    assert!(header_len <= PayloadLength::MAX as usize);
    let total_packet_len = header_len + PAYLOAD_LEN_BYTES + HEADER_LEN_BYTES;

    buf = write_le!(HeaderLength, header_len, buf);
    write_le!(PayloadLength, payload_len, buf);
    Ok(total_packet_len)
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
    pub const PACKET_LEN_BUF_SIZE: usize = PAYLOAD_LEN_BYTES + HEADER_LEN_BYTES;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_message_len<I>(&self, bytes: &mut I) -> (usize, usize)
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        assert!(bytes.len() >= Self::PACKET_LEN_BUF_SIZE);

        let header_len = read_le!(HeaderLength, &mut bytes) as usize;
        let payload_len = read_le!(PayloadLength, &mut bytes) as usize;

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
            MessageHeader::ClearImages => Message::ClearImages,
            MessageHeader::AddImage(img) => Message::AddImage {
                refresh_rate: img.refresh_rate,
                bytes,
                strip_len: img.strip_len as usize,
            },
            MessageHeader::ShowImage(index) => Message::ShowImage {
                index: index as usize,
            },

            MessageHeader::Info(info) => Message::Info(info),
            MessageHeader::Ok => Message::Ok,
            MessageHeader::Error(code) => Message::Error(Error::from_code(code)),
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
    ShowImage {
        index: usize,
    },
    ClearImages,

    // Responses.
    Ok,
    ImageAdded {
        index: usize,
    },
    Info(FirmwareInfo),
    Error(Error),
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
                    bytes_len: bytes.len() as u32,
                }),
                Some(bytes),
            ),
            Message::ClearImages => (MessageHeader::ClearImages, None),
            Message::ShowImage { index } => (MessageHeader::ShowImage(index as u16), None),

            Message::ImageAdded { index } => (MessageHeader::ImageAdded(index as u16), None),
            Message::Ok => (MessageHeader::Ok, None),
            Message::Info(info) => (MessageHeader::Info(info), None),
            Message::Error(err) => (MessageHeader::Error(err.into_code()), None),
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

pub type SimpleMessage = Message<Empty<u8>>;

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

/// Errors that can occur when processing messages.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Debug)]
pub enum Error {
    /// The length of the strip does not match with the specified.
    StripLengthMismatch,
    /// The length of the picture in bytes is not a multiple of "strip length" * "bytes per pixel".
    ImageLengthMismatch,
    /// The transmitted message cannot be fitted into the device's memory.
    ImageTooBig,
    /// This image repository on the device is full.
    ImageRepositoryFull,
    /// The specified image index is greater than the total amount of the stored images.
    ImageNotFound,
    /// Unexpected response to the request.
    UnexpectedResponse,
    /// Unspecified or unknown error.
    Unspecified(u16),
}

impl Error {
    pub(crate) fn from_code(code: u16) -> Self {
        match code {
            1 => Self::StripLengthMismatch,
            2 => Self::ImageLengthMismatch,
            3 => Self::ImageTooBig,
            4 => Self::ImageRepositoryFull,
            5 => Self::ImageNotFound,
            6 => Self::UnexpectedResponse,
            other => Self::Unspecified(other),
        }
    }

    pub(crate) fn into_code(self) -> u16 {
        match self {
            Error::StripLengthMismatch => 1,
            Error::ImageLengthMismatch => 2,
            Error::ImageTooBig => 3,
            Error::ImageRepositoryFull => 4,
            Error::ImageNotFound => 5,
            Error::UnexpectedResponse => 6,
            Error::Unspecified(other) => other,
        }
    }
}

impl From<Error> for SimpleMessage {
    fn from(err: Error) -> Self {
        SimpleMessage::Error(err)
    }
}
