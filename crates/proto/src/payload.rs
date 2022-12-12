//! Payload reader

use embedded_io::{
    blocking::{Read, Seek},
    Io,
};

use crate::ExactSizedRead;

#[derive(Debug, Clone, Copy)]
pub struct PayloadReader<T> {
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

impl<T: Seek + Read> Seek for PayloadReader<T> {
    fn seek(&mut self, pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
        todo!()
    }
}

impl<T: Read> ExactSizedRead for PayloadReader<T> {
    fn bytes_len(&self) -> usize {
        self.payload_len
    }

    fn bytes_remaining(&self) -> usize {
        self.bytes_remaining
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
