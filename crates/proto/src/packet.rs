use core::fmt::Debug;

use embedded_io::blocking::{Read, ReadExactError};
pub use endian_codec::PackedSize;
use endian_codec::{DecodeLE, EncodeLE};

use crate::headers::MessageHeader;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct Packet {
    pub header_len: u32,
    pub payload_len: u32,
}

impl Packet {
    pub fn header_len(self) -> usize {
        self.header_len as usize
    }

    pub fn payload_len(self) -> usize {
        self.payload_len as usize
    }
}

#[derive(Debug, displaydoc::Display)]
pub enum PacketReadError<E> {
    /// Unable to decode message
    Decode(postcard::Error),
    /// Unexpected end of file
    UnexpectedEof,
    /// Other error
    Other(E),
}

// TODO Temporary until I implement proper error types.
impl<E: Debug> std::error::Error for PacketReadError<E> {}

impl<E: embedded_io::Error> From<ReadExactError<E>> for PacketReadError<E> {
    fn from(inner: ReadExactError<E>) -> Self {
        match inner {
            ReadExactError::UnexpectedEof => Self::UnexpectedEof,
            ReadExactError::Other(other) => Self::Other(other),
        }
    }
}

impl<E: embedded_io::Error> From<postcard::Error> for PacketReadError<E> {
    fn from(inner: postcard::Error) -> Self {
        Self::Decode(inner)
    }
}

impl Packet {
    pub fn read<T: Read>(mut reader: T) -> Result<Self, PacketReadError<T::Error>> {
        let mut buf = [0_u8; Packet::PACKED_LEN];
        reader.read_exact(&mut buf)?;
        Ok(Packet::decode_from_le_bytes(&buf))
    }

    pub fn message<T: Read>(
        self,
        mut reader: T,
    ) -> Result<(MessageHeader, usize), PacketReadError<T::Error>> {
        let mut buf = [0_u8; MessageHeader::MAX_LEN];

        let header_buf = &mut buf[0..self.header_len()];
        reader.read_exact(header_buf)?;
        let header = postcard::from_bytes(header_buf)?;
        Ok((header, self.payload_len()))
    }
}

impl MessageHeader {
    pub fn encode<'a>(&self, buf: &'a mut [u8], payload_len: usize) -> &'a mut [u8] {
        assert!(buf.len() >= MessageHeader::MAX_LEN);

        let message_buf = &mut buf[Packet::PACKED_LEN..];
        let header_len = postcard::to_slice(self, message_buf).unwrap().len();

        let packet = Packet {
            header_len: header_len as u32,
            payload_len: payload_len as u32,
        };
        packet.encode_as_le_bytes(&mut buf[..Packet::PACKED_LEN]);

        &mut buf[..Packet::PACKED_LEN + header_len]
    }
}
