//! Low level connection between cyberpixie devices.

use std::{
    io::{ErrorKind, Read, Write},
    net::TcpStream,
};

use cyberpixie_proto::ng::{
    transport::{PackedSize, Packet},
    MessageHeader, PayloadReader, DeviceRole,
};
use embedded_io::adapters::{FromStd, ToStd};
use log::trace;
use nb_utils::IntoNbResult;

pub struct TcpStreamReader<'a>(embedded_io::adapters::FromStd<&'a mut TcpStream>);

impl<'a> TcpStreamReader<'a> {
    pub fn new(inner: &'a mut TcpStream) -> Self {
        Self(FromStd::new(inner))
    }
}

impl<'a> embedded_io::Io for TcpStreamReader<'a> {
    type Error = std::io::Error;
}

impl<'a> embedded_io::blocking::Read for TcpStreamReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        nb::block!(self.0.read(buf).into_nb_result())
    }
}

pub struct Message<R: embedded_io::blocking::Read> {
    pub header: MessageHeader,
    pub payload: Option<PayloadReader<R>>,
}

pub type IncomingMessage<'a> = Message<TcpStreamReader<'a>>;

impl<R: embedded_io::blocking::Read> Message<R> {
    pub fn send<W: Write>(self, mut device: W) -> Result<(), std::io::Error> {
        // TODO Move this const.
        const SEND_BUF_LEN: usize = 256;

        let (header, payload_len, reader) = self.into_parts();

        let mut send_buf = [0_u8; SEND_BUF_LEN];
        let header_buf = header.encode(&mut send_buf, payload_len);
        device.write_all(header_buf)?;

        if let Some(mut reader) = reader {
            loop {
                let bytes_read = reader.read(&mut send_buf)?;
                if bytes_read == 0 {
                    break;
                }
                device.write_all(&send_buf[0..bytes_read])?;
            }
        }
        Ok(())
    }

    pub fn into_parts(self) -> (MessageHeader, usize, Option<ToStd<PayloadReader<R>>>) {
        if let Some(reader) = self.payload {
            (self.header, reader.len(), Some(ToStd::new(reader)))
        } else {
            (self.header, 0, None)
        }
    }

    pub fn read_payload_to_vec(self) -> Result<Vec<u8>, anyhow::Error> {
        let payload = self.payload.expect("There is no payload in this message");
        let mut buf = vec![0_u8; payload.len()];
        ToStd::new(payload).read_exact(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Debug)]
pub struct Connection {
    role: DeviceRole,
    stream: TcpStream,
    packet_header_buf: heapless::Vec<u8, { Packet::PACKED_LEN }>,
}

impl Connection {
    pub fn new(stream: TcpStream, role: DeviceRole) -> Self {
        stream.set_nodelay(true).ok();
        stream.set_nonblocking(true).ok();
        Self {
            stream,
            packet_header_buf: Default::default(),
            role,
        }
    }

    pub fn poll_next_message(&mut self) -> nb::Result<IncomingMessage<'_>, anyhow::Error> {
        let (header, payload_len) = self
            .poll_next_packet()?
            .message(self.io_reader())
            .map_err(|x| nb::Error::Other(anyhow::anyhow!("{x}")))?;
        trace!("[{}] Got a next message {header:?}, {payload_len:?}", self.role);

        let payload = if payload_len > 0 {
            Some(PayloadReader::new(self.io_reader(), payload_len))
        } else {
            None
        };

        Ok(Message { header, payload })
    }

    pub fn send_message(&mut self, header: MessageHeader) -> anyhow::Result<()> {
        trace!("[{}] Sending message {header:?}", self.role);

        Message::<&[u8]> {
            header,
            payload: None,
        }
        .send(&mut self.stream)?;
        Ok(())
    }

    pub fn send_message_with_payload<T, P>(
        &mut self,
        header: MessageHeader,
        payload: P,
    ) -> anyhow::Result<()>
    where
        T: embedded_io::blocking::Read,
        P: Into<PayloadReader<T>>,
    {
        let payload = payload.into();
        trace!(
            "[{}] Sending message {header:?} with payload {}",
            self.role,
            payload.len()
        );

        Message {
            header,
            payload: Some(payload),
        }
        .send(&mut self.stream)?;
        Ok(())
    }

    fn poll_next_packet(&mut self) -> nb::Result<Packet, anyhow::Error> {
        let mut buf = [0_u8; Packet::PACKED_LEN];

        let bytes_remaining = Packet::PACKED_LEN - self.packet_header_buf.len();
        let bytes_read = match self.stream.read(&mut buf[0..bytes_remaining]) {
            // Successfuly read bytes.
            Ok(bytes_read) if bytes_read > 0 => Ok(bytes_read),
            // Various blocking situations
            Ok(_) => Err(nb::Error::WouldBlock),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Err(nb::Error::WouldBlock),
            // Something went wrong
            Err(err) => Err(nb::Error::Other(err.into())),
        }?;

        self.packet_header_buf
            .extend_from_slice(&buf[..bytes_read])
            .unwrap();

        if self.packet_header_buf.is_full() {
            let mut buf: &[u8] = &self.packet_header_buf;
            let packet = Packet::read(&mut buf)
                .map_err(|err| nb::Error::Other(anyhow::anyhow!("{}", err)))?;
            trace!("[{}] Got a next packet {packet:?}", self.role);

            self.packet_header_buf.clear();
            Ok(packet)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn io_reader(&mut self) -> TcpStreamReader<'_> {
        TcpStreamReader::new(&mut self.stream)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{TcpListener, TcpStream};

    use cyberpixie_proto::ng::{DeviceInfo, DeviceRole, MessageHeader};
    use nb_utils::{IntoNbResult, NbResultExt};

    use super::Connection;

    fn create_loopback() -> (Connection, Connection) {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let sender = TcpStream::connect(addr).unwrap();
        (
            Connection::new(sender, DeviceRole::Client),
            Connection::new(listener.accept().unwrap().0, DeviceRole::Main),
        )
    }

    #[test]
    fn test_create_loop_back_nonblocking() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();

        assert!(listener.accept().into_nb_result().is_would_block());

        // Accept a first stream
        let stream = TcpStream::connect(addr).unwrap();
        nb::block!(listener.accept().into_nb_result()).unwrap();
        assert!(listener.accept().into_nb_result().is_would_block());
        drop(stream);
        // Accept a second stream as well
        let stream = TcpStream::connect(addr).unwrap();
        nb::block!(listener.accept().into_nb_result()).unwrap();
        assert!(listener.accept().into_nb_result().is_would_block());
        drop(stream);

        assert!(listener.accept().into_nb_result().is_would_block());
    }

    #[test]
    fn test_read_write_without_payload() {
        let (mut sender, mut receiver) = create_loopback();

        assert!(receiver.poll_next_packet().is_would_block());

        let message = MessageHeader::RequestHandshake(DeviceInfo {
            role: DeviceRole::Client,
            group_id: None,
            strip_len: 64,
        });
        sender.send_message(message).unwrap();

        let next_message = nb::block!(receiver.poll_next_message()).unwrap();
        assert_eq!(next_message.header, message);
        assert!(next_message.payload.is_none());

        assert!(receiver.poll_next_packet().is_would_block());
    }

    #[test]
    fn test_read_write_with_payload() {
        let (mut sender, mut receiver) = create_loopback();

        let text = b"Hello cyberpixie".as_slice();
        sender
            .send_message_with_payload(MessageHeader::Debug, text)
            .unwrap();

        let next_message = nb::block!(receiver.poll_next_message()).unwrap();
        assert_eq!(next_message.header, MessageHeader::Debug);
        let text2 = next_message.read_payload_to_vec().unwrap();
        assert_eq!(text, text2);

        assert!(receiver.poll_next_packet().is_would_block());
    }
}
