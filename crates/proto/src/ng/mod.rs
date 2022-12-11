use embedded_io::{blocking::Read, Io};

pub use messages::MessageHeader;
pub use types::{DeviceRole, FirmwareInfo, DeviceInfo, Hertz, ImageId, ImageInfo};

mod messages;
pub mod transport;
mod types;

#[derive(Debug, Clone, Copy)]
pub struct PayloadReader<T: Read> {
    payload_len: usize,
    bytes_remaining: usize,
    inner: T,
}

impl<T: Read> PayloadReader<T> {
    pub fn new(inner: T, payload_len: usize) -> Self {
        Self {
            payload_len,
            bytes_remaining: payload_len,
            inner,
        }
    }

    pub fn len(&self) -> usize {
        self.payload_len
    }

    pub fn is_empty(&self) -> bool {
        self.payload_len == 0
    }
}

impl<T: Read> Io for PayloadReader<T> {
    type Error = T::Error;
}

impl<T: Read> Read for PayloadReader<T> {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Don't read more bytes the from buffer than necessary.
        if buf.len() > self.bytes_remaining {
            buf = &mut buf[0..self.bytes_remaining];
        }

        let bytes_read = self.inner.read(buf)?;
        self.bytes_remaining -= bytes_read;
        Ok(bytes_read)
    }
}

impl<'a> From<&'a [u8]> for PayloadReader<&'a [u8]> {
    fn from(inner: &'a [u8]) -> Self {
        Self::new(inner, inner.len())
    }
}

impl<'a> From<&'a str> for PayloadReader<&'a [u8]> {
    fn from(inner: &'a str) -> Self {
        Self::new(inner.as_bytes(), inner.len())
    }
}

pub trait Peer {
    type Address;
}

pub trait Service: Peer {
    fn handle_connect(
        &mut self,
        peer: Self::Address,
        handshake: DeviceInfo,
    ) -> Result<DeviceInfo, anyhow::Error>;

    // fn handle_disconnect(&mut self, peer: Self::Address) -> Result<(), anyhow::Error>;

    // fn handle_add_image<T: Read>(
    //     &mut self,
    //     peer: Self::Address,
    //     info: ImageInfo,
    //     body: PayloadReader<T>,
    // ) -> Result<ImageId, anyhow::Error>;
}

pub trait Client: Peer {
    fn connect(
        &mut self,
        peer: Self::Address,
        handshake: DeviceInfo,
    ) -> Result<DeviceInfo, anyhow::Error>;

    // fn add_image<T: Read>(
    //     &mut self,
    //     peer: Self::Address,
    //     info: ImageInfo,
    //     body: PayloadReader<T>,
    // ) -> Result<ImageId, anyhow::Error>;

    // fn disconnect(&mut self, peer: Self::Address) -> Result<(), anyhow::Error>;
}
