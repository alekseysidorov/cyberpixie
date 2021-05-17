#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{
    fmt::Display,
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::Path,
};

use cyberpixie_proto::{
    types::Hertz, Message, PacketReader, Service, ServiceEvent, MAX_HEADER_LEN,
};
use image::io::Reader;

struct ServiceImpl {
    next_msg: Vec<u8>,
    stream: TcpStream,
}

impl ServiceImpl {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            next_msg: Vec::new(),
        }
    }
}

struct BytesIter<'a> {
    vec: &'a mut Vec<u8>,
    pos: usize,
}

impl<'a> Iterator for BytesIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.vec.len() {
            self.pos = 0;
            self.vec.clear();
            return None;
        }

        let byte = self.vec[self.pos];
        self.pos += 1;
        Some(byte)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BytesIter<'a> {}

impl<'a> Drop for BytesIter<'a> {
    fn drop(&mut self) {
        self.vec.clear();
    }
}

impl Service for ServiceImpl {
    type Error = anyhow::Error;

    type Address = ();

    type BytesReader<'a> = BytesIter<'a>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<ServiceEvent<Self::Address, Self::BytesReader<'_>>, Self::Error> {
        let mut read_buf = [0u8; MAX_HEADER_LEN];

        let bytes_read = match self.stream.read(&mut read_buf) {
            Ok(bytes_read) if bytes_read > 0 => Ok(bytes_read),
            Err(err) if err.kind() != io::ErrorKind::Interrupted => {
                Err(nb::Error::Other(Self::Error::from(err)))
            }
            _ => Err(nb::Error::WouldBlock),
        }?;
        self.next_msg.extend_from_slice(&read_buf[0..bytes_read]);

        log::debug!(
            "Got {} bytes, next_msg_len: {} (should be {})",
            bytes_read,
            self.next_msg.len(),
            PacketReader::PACKET_LEN_BUF_SIZE
        );

        if self.next_msg.len() < PacketReader::PACKET_LEN_BUF_SIZE {
            return Err(nb::Error::WouldBlock);
        }

        log::debug!("Reading message...");

        let mut reader = PacketReader::new();
        let (header_len, payload_len) = reader.read_message_len(
            &mut self.next_msg[0..PacketReader::PACKET_LEN_BUF_SIZE]
                .iter()
                .copied(),
        );
        log::debug!(
            "Got packet sizes, hdr: {}, pld: {}",
            header_len,
            payload_len
        );

        let total_len = header_len + payload_len + PacketReader::PACKET_LEN_BUF_SIZE;
        if self.next_msg.len() < total_len {
            // Now we know exactly how many bytes we must read from the stream
            // to get the message.
            self.next_msg.resize(header_len + payload_len, 0);
            self.stream
                .read_exact(&mut self.next_msg[header_len + PacketReader::PACKET_LEN_BUF_SIZE..])
                .map_err(Self::Error::from)
                .map_err(nb::Error::Other)?;
        }

        log::debug!("Message bytes read");

        let read_iter = BytesIter {
            vec: &mut self.next_msg,
            pos: PacketReader::PACKET_LEN_BUF_SIZE,
        };

        let msg = reader
            .read_message(read_iter, header_len)
            .expect("Unable to decode message");

        Ok(ServiceEvent::Data {
            address: (),
            payload: msg,
        })
    }

    fn send_message<I>(
        &mut self,
        _to: Self::Address,
        message: Message<I>,
    ) -> Result<(), Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let next_msg = message.into_bytes().collect::<Vec<_>>();
        self.stream.write_all(&next_msg).map_err(From::from)
    }
}

pub fn convert_image_to_raw(path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
    let image = Reader::open(path)?.decode()?.to_rgb8();

    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }

    Ok(raw)
}

fn no_response() -> anyhow::Error {
    anyhow::format_err!("Expected response from the device")
}

pub fn send_image<T: ToSocketAddrs + Display + Copy>(
    strip_len: usize,
    refresh_rate: Hertz,
    raw: Vec<u8>,
    to: T,
) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(TcpStream::connect(to)?);

    let index = service
        .add_image((), refresh_rate, strip_len, raw.into_iter())?
        .ok_or_else(no_response)?;
    log::trace!("Sent image to {}, image index is: {}", to, index);
    Ok(())
}

pub fn send_clear_images<T: ToSocketAddrs + Display + Copy>(to: T) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(TcpStream::connect(to)?);

    service.clear_images(())?.ok_or_else(no_response)?;
    log::trace!("Sent images clear command to {}", to);
    Ok(())
}

pub fn show_image<T: ToSocketAddrs + Display + Copy>(index: usize, to: T) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(TcpStream::connect(to)?);

    service.show_image((), index)?.ok_or_else(no_response)?;
    log::trace!("Sent show image {} command to {}", index, to);
    Ok(())
}
