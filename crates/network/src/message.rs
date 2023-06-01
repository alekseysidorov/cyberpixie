//! Cyberpixie protocol messages io adapters

use cyberpixie_core::{proto::Headers, ExactSizeRead};

// FIXME?
const SEND_BUF_LEN: usize = 256;

/// Incoming message payload reader.
#[derive(Debug, Clone, Copy)]
pub struct PayloadReader<T> {
    payload_len: usize,
    bytes_remaining: usize,
    inner: T,
}

impl<T> PayloadReader<T> {
    /// Creates a new message payload reader.
    pub(crate) const fn new(inner: T, payload_len: usize) -> Self {
        Self {
            payload_len,
            bytes_remaining: payload_len,
            inner,
        }
    }

    /// Returns payload length.
    pub const fn len(&self) -> usize {
        self.payload_len
    }

    /// Returns true if there is no payload.
    pub const fn is_empty(&self) -> bool {
        self.payload_len == 0
    }
}

impl<T: embedded_io::Io> embedded_io::Io for PayloadReader<T> {
    type Error = T::Error;
}

impl<T> ExactSizeRead for PayloadReader<T> {
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

impl<T: embedded_io::blocking::Read> embedded_io::blocking::Read for PayloadReader<T> {
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

impl<T: embedded_io::asynch::Read> embedded_io::asynch::Read for PayloadReader<T> {
    async fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, Self::Error> {
        // Don't read more bytes the from buffer than necessary.
        if buf.len() > self.bytes_remaining {
            buf = &mut buf[0..self.bytes_remaining];
        }

        let bytes_read = self.inner.read(buf).await?;
        self.bytes_remaining -= bytes_read;
        Ok(bytes_read)
    }
}

/// Cyberpixie protocol message io adapter.
///
/// This adapter can be used both for receiving messages and for sending, depending on the goals.
pub struct Message<R, H> {
    /// Message header.
    pub header: H,
    /// Message payload.
    pub payload: Option<PayloadReader<R>>,
}

impl<R, H> Message<R, H> {
    fn into_parts(self) -> (H, usize, Option<PayloadReader<R>>) {
        if let Some(reader) = self.payload {
            (self.header, reader.len(), Some(reader))
        } else {
            (self.header, 0, None)
        }
    }
}

impl<R: embedded_io::blocking::Read> Message<R, Headers> {
    pub fn send_blocking<W>(self, mut device: W) -> cyberpixie_core::Result<()>
    where
        W: embedded_io::blocking::Write,
    {
        use embedded_io::blocking::Read;

        let (header, payload_len, payload_reader) = self.into_parts();

        let mut send_buf = [0_u8; SEND_BUF_LEN];
        let header_buf = header.encode(&mut send_buf, payload_len);
        device
            .write_all(header_buf)
            .map_err(cyberpixie_core::Error::network)?;

        if let Some(mut reader) = payload_reader {
            loop {
                let bytes_read = reader
                    .read(&mut send_buf)
                    .map_err(cyberpixie_core::Error::storage_read)?;
                if bytes_read == 0 {
                    break;
                }
                device
                    .write_all(&send_buf[0..bytes_read])
                    .map_err(cyberpixie_core::Error::network)?;
            }
        }
        Ok(())
    }
}

impl<R: embedded_io::blocking::Read> Message<R, Headers> {
    pub async fn send<W>(self, mut device: W) -> cyberpixie_core::Result<()>
    where
        W: embedded_io::asynch::Write,
    {
        use embedded_io::blocking::Read;

        let (header, payload_len, payload_reader) = self.into_parts();

        let mut send_buf = [0_u8; SEND_BUF_LEN];
        let header_buf = header.encode(&mut send_buf, payload_len);
        device
            .write_all(header_buf)
            .await
            .map_err(cyberpixie_core::Error::network)?;

        if let Some(mut reader) = payload_reader {
            loop {
                let bytes_read = reader
                    .read(&mut send_buf)
                    .map_err(cyberpixie_core::Error::storage_read)?;
                if bytes_read == 0 {
                    break;
                }
                device
                    .write_all(&send_buf[0..bytes_read])
                    .await
                    .map_err(cyberpixie_core::Error::network)?;
            }
        }
        Ok(())
    }
}
