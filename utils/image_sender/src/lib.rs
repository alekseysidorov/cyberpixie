#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{
    fmt::Display,
    io::{self, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::Path,
};

use cyberpixie_proto::{
    types::{Hertz, MessageHeader},
    write_message_header, Message, PacketReader, Service, ServiceEvent, MAX_HEADER_LEN,
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

    fn poll_next(
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

        if self.next_msg.len() > PacketReader::PACKET_LEN_BUF_SIZE {
            return Err(nb::Error::WouldBlock);
        }

        let mut reader = PacketReader::new();
        let (header_len, payload_len) = reader.read_message_len(
            &mut self.next_msg[0..PacketReader::PACKET_LEN_BUF_SIZE]
                .iter()
                .copied(),
        );
        // Now we know exactly how many bytes we must read from the stream
        // to get the message.
        self.next_msg.resize(header_len + payload_len, 0);
        self.stream
            .read_exact(&mut self.next_msg)
            .map_err(Self::Error::from)
            .map_err(nb::Error::Other)?;

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

pub fn send_image<T: ToSocketAddrs + Display + Copy>(
    strip_len: usize,
    refresh_rate: Hertz,
    raw: Vec<u8>,
    to: T,
) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(TcpStream::connect(to)?);
    service.send_message(
        (),
        Message::AddImage {
            refresh_rate,
            strip_len,
            bytes: raw.into_iter(),
        },
    )?;
    log::trace!("Sent image to {}", to);

    let response = nb::block!(service.poll_next())?;
    if let ServiceEvent::Data {
        payload: Message::ImageAdded { index },
        ..
    } = response
    {
        log::info!("Message index is {}", index)
    }

    Ok(())
}

pub fn send_clear_images<T: ToSocketAddrs + Display + Copy>(to: T) -> anyhow::Result<()> {
    let mut header_buf = vec![0_u8; MAX_HEADER_LEN];
    let msg = MessageHeader::ClearImages;

    let total_len = write_message_header(&mut header_buf, &msg, 0)
        .map_err(|e| anyhow::format_err!("Unable to write message header: {:?}", e))?;
    header_buf.truncate(total_len);

    let mut stream = TcpStream::connect(to)?;
    stream.write_all(&header_buf)?;

    log::trace!("Sent reset cmd to {}: {:?}", to, msg);
    Ok(())
}
