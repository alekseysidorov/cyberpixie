#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{
    fmt::Display,
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use cyberpixie_proto::{
    types::Hertz, Message, PacketReader, Service, ServiceEvent, MAX_HEADER_LEN,
};
use image::io::Reader;

const TIMEOUT: Duration = Duration::from_secs(15);

struct ServiceImpl {
    next_msg: Vec<u8>,
    stream: TcpStream,
}

impl ServiceImpl {
    pub fn new(addr: &SocketAddr) -> anyhow::Result<Self> {
        log::debug!("Connecting to the {}", addr);
        let stream = TcpStream::connect_timeout(addr, TIMEOUT)?;
        log::debug!("Connected");

        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        stream.set_nodelay(true).ok();

        Ok(Self {
            stream,
            next_msg: Vec::new(),
        })
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
        log::debug!("Sending message...");

        let next_msg = message.into_bytes().collect::<Vec<_>>();
        self.stream.write_all(&next_msg)?;

        log::debug!("Message sent");
        Ok(())
    }
}

pub fn convert_image_to_raw(path: impl AsRef<Path>) -> anyhow::Result<(usize, Vec<u8>)> {
    let image = Reader::open(path)?.decode()?.to_rgb8();
    let width = image.width() as usize;

    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }

    Ok((width, raw))
}

fn display_err(err: impl Display) -> anyhow::Error {
    anyhow::format_err!("Expected response from the device: {}", err)
}

pub fn send_image(
    strip_len: usize,
    refresh_rate: Hertz,
    raw: Vec<u8>,
    to: SocketAddr,
) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(&to)?;

    let index = service
        .add_image((), refresh_rate, strip_len, raw.into_iter())?
        .map_err(display_err)?;
    log::info!("Image loaded into the device {} with index {}", to, index);
    Ok(())
}

pub fn send_clear_images(to: SocketAddr) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(&to)?;

    service.clear_images(())?.map_err(display_err)?;
    log::trace!("Sent images clear command to {}", to);
    Ok(())
}

pub fn send_show_image(index: usize, to: SocketAddr) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(&to)?;

    service.show_image((), index)?.map_err(display_err)?;
    log::trace!("Showing image at {} on device {}", index, to);
    Ok(())
}

pub fn send_firmware_info(to: SocketAddr) -> anyhow::Result<()> {
    let mut service = ServiceImpl::new(&to)?;

    let info = service.request_firmware_info(())?.map_err(display_err)?;
    log::info!("Got {:?} from {}", info, to);
    Ok(())
}
