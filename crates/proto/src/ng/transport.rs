use embedded_io::{
    blocking::{Read, ReadExactError},
    Io,
};
use endian_codec::{DecodeLE, EncodeLE, PackedSize};

use super::{messages::MessageHeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct Packet {
    pub header_len: u16,
    pub payload_len: u16,
}

impl Packet {
    pub fn header_len(self) -> usize {
        self.header_len as usize
    }

    pub fn payload_len(self) -> usize {
        self.payload_len as usize
    }
}

pub enum PacketReadError<T: Io> {
    Decode(postcard::Error),
    UnexpectedEof,
    Other(T::Error),
}

impl<T: Io> From<ReadExactError<T::Error>> for PacketReadError<T> {
    fn from(inner: ReadExactError<T::Error>) -> Self {
        match inner {
            ReadExactError::UnexpectedEof => Self::UnexpectedEof,
            ReadExactError::Other(other) => Self::Other(other),
        }
    }
}

impl<T: Io> From<postcard::Error> for PacketReadError<T> {
    fn from(inner: postcard::Error) -> Self {
        Self::Decode(inner)
    }
}


impl Packet {
    pub fn read<T: Read>(reader: &mut T) -> Result<Self, PacketReadError<T>> {
        let mut buf = [0_u8; Packet::PACKED_LEN];
        reader.read_exact(&mut buf)?;
        Ok(Packet::decode_from_le_bytes(&buf))
    }

    pub fn message<T: Read>(self, reader: &mut T) -> Result<(MessageHeader, usize), PacketReadError<T>> {
        let mut buf = [0_u8; MessageHeader::MAX_SIZE];

        let header_buf = &mut buf[0..self.header_len()];
        reader.read_exact(header_buf)?;
        let header = postcard::from_bytes(header_buf)?;
        Ok((header, self.payload_len()))
    }
}
