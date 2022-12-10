use std::{
    io::{ErrorKind, Read},
    net::{TcpListener, TcpStream},
    time::Duration,
};

use cyberpixie_proto::ng::{
    transport::{PackedSize, Packet},
    MessageHeader, PayloadReader,
};
use embedded_io::adapters::FromStd;

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
        loop {
            let result = self.0.read(buf);
            if matches!(&result, Err(err) if err.kind() == ErrorKind::WouldBlock) {
                continue;
            }

            return result;
        }
    }
}

pub type TcpPayloadReader<'a> = PayloadReader<TcpStreamReader<'a>>;

pub struct NextMessage<'a> {
    pub header: MessageHeader,
    pub payload: Option<TcpPayloadReader<'a>>,
}

pub struct Connection {
    stream: TcpStream,
    packet_header_buf: heapless::Vec<u8, { Packet::PACKED_LEN }>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        stream.set_nodelay(true).ok();
        stream.set_nonblocking(true).ok();
        Self {
            stream,
            packet_header_buf: Default::default(),
        }
    }

    pub fn poll_next_message(&mut self) -> nb::Result<NextMessage<'_>, anyhow::Error> {
        let (header, payload_len) = self
            .poll_next_packet()?
            .message(self.io_reader())
            .map_err(|x| nb::Error::Other(anyhow::anyhow!("{x}")))?;

        let payload = if payload_len > 0 {
            Some(PayloadReader {
                len: payload_len,
                inner: self.io_reader(),
            })
        } else {
            None
        };

        Ok(NextMessage { header, payload })
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
            Ok(packet)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn io_reader(&mut self) -> TcpStreamReader<'_> {
        TcpStreamReader::new(&mut self.stream)
    }
}

// impl TcpTransport {
//     pub fn new(address: SocketAddr, stream: TcpStream) -> Self {
//         // TODO rewrite on tokio.
//         Self {
//             address,
//             stream,
//             next_msg: Vec::new(),
//         }
//     }

//     fn read_packet_kind(&mut self) -> nb::Result<PacketKind, anyhow::Error> {
//         let mut msg_buf = [0_u8; PacketKind::PACKED_LEN];

//         let bytes_read = match self.stream.read(&mut msg_buf) {
//             Ok(bytes_read) if bytes_read > 0 => Ok(bytes_read),
//             Err(err) => Err(nb::Error::Other(anyhow::Error::from(err))),
//             _ => Err(nb::Error::WouldBlock),
//         }?;
//         self.next_msg.extend_from_slice(&msg_buf[..bytes_read]);

//         if self.next_msg.len() >= PacketKind::PACKED_LEN {
//             Ok(PacketKind::decode(&self.next_msg))
//         } else {
//             Err(nb::Error::WouldBlock)
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        net::{TcpListener, TcpStream},
    };

    use cyberpixie_proto::ng::{DeviceRole, Handshake, MessageHeader};
    use embedded_io::blocking::Read;

    use super::TcpConnection;

    fn create_loopback() -> (TcpStream, TcpListener) {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let sender = TcpStream::connect(addr).unwrap();
        (sender, listener)
    }

    #[test]
    fn test_read_write_without_payload() {
        let (mut sender, listener) = create_loopback();
        let mut connection = TcpConnection::new(listener.accept().unwrap().0);

        connection.poll_next_packet().unwrap_err();

        let mut buf = [0_u8; MessageHeader::MAX_LEN];
        let message = MessageHeader::RequestHandshake(Handshake {
            role: DeviceRole::Host,
            group_id: None,
            strip_len: 64,
        });
        let buf = message.encode(&mut buf, 0);
        sender.write_all(buf).unwrap();

        let next_message = nb::block!(connection.poll_next_message()).unwrap();
        assert_eq!(next_message.header, message);
        assert!(next_message.payload.is_none());
    }

    #[test]
    fn test_read_write_with_payload() {
        let (mut sender, listener) = create_loopback();
        let mut connection = TcpConnection::new(listener.accept().unwrap().0);

        let mut buf = [0_u8; MessageHeader::MAX_LEN];

        let text = "Hello cyberpixie";
        let buf = MessageHeader::Debug.encode(&mut buf, text.len());
        sender.write_all(buf).unwrap();
        sender.write_all(text.as_bytes()).unwrap();

        let next_message = nb::block!(connection.poll_next_message()).unwrap();
        assert_eq!(next_message.header, MessageHeader::Debug);

        let mut reader = next_message.payload.unwrap();
        let mut buf = vec![0_u8; reader.len];
        reader.inner.read(&mut buf).unwrap();
        let text2 = String::from_utf8(buf).unwrap();

        assert_eq!(text, text2);

        connection.poll_next_packet().unwrap_err();
    }
}
