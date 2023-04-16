use core::fmt::Debug;

use embedded_io::blocking::{Read, ReadExactError};
pub use endian_codec::PackedSize;
use endian_codec::{DecodeLE, EncodeLE};
use postcard::experimental::max_size::MaxSize;

use super::{Headers, RequestHeader, ResponseHeader};

/// Max packet with header lenght.
const MAX_LEN: usize = Headers::POSTCARD_MAX_SIZE + Packet::PACKED_LEN;

pub trait FromPacket: Sized {
    fn from_packet<R: Read>(
        packet: Packet,
        reader: R,
    ) -> Result<(Self, usize), PacketReadError<R::Error>>;
}

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
    /// Unable to decode message: {0}.
    Decode(postcard::Error),
    /// Unexpected end of file
    UnexpectedEof,
    /// Other error
    Other(E),
}

#[cfg(feature = "std")]
impl<E: Debug> std::error::Error for PacketReadError<E> {}

#[cfg(feature = "std")]
impl<E: std::fmt::Display> From<PacketReadError<E>> for std::io::Error {
    fn from(err: PacketReadError<E>) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
    }
}

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

    pub fn header<R: Read, H: FromPacket>(
        self,
        reader: R,
    ) -> Result<(H, usize), PacketReadError<R::Error>> {
        H::from_packet(self, reader)
    }
}

impl RequestHeader {
    pub fn encode(self, buf: &mut [u8], payload_len: usize) -> &mut [u8] {
        Headers::Request(self).encode(buf, payload_len)
    }
}

impl FromPacket for RequestHeader {
    fn from_packet<R: Read>(
        packet: Packet,
        mut reader: R,
    ) -> Result<(RequestHeader, usize), PacketReadError<R::Error>> {
        let mut buf = [0_u8; MAX_LEN];

        let header_buf = &mut buf[0..packet.header_len()];
        reader.read_exact(header_buf)?;
        let header = postcard::from_bytes(header_buf)?;
        Ok((header, packet.payload_len()))
    }
}

impl ResponseHeader {
    pub fn encode(self, buf: &mut [u8], payload_len: usize) -> &mut [u8] {
        Headers::Response(self).encode(buf, payload_len)
    }
}

impl FromPacket for ResponseHeader {
    fn from_packet<R: Read>(
        packet: Packet,
        mut reader: R,
    ) -> Result<(ResponseHeader, usize), PacketReadError<R::Error>> {
        let mut buf = [0_u8; MAX_LEN];

        let header_buf = &mut buf[0..packet.header_len()];
        reader.read_exact(header_buf)?;
        let header = postcard::from_bytes(header_buf)?;
        Ok((header, packet.payload_len()))
    }
}

impl Headers {
    pub fn encode<'a>(&self, buf: &'a mut [u8], payload_len: usize) -> &'a mut [u8] {
        assert!(buf.len() >= MAX_LEN);

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
