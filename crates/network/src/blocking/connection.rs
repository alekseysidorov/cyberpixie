//! A connection between Cybeprixie peers.

use core::fmt::Debug;

use cyberpixie_core::proto::{
    packet::{FromPacket, PackedSize, Packet},
    Headers, RequestHeader, ResponseHeader,
};
use embedded_io::blocking::Read;
use embedded_nal::TcpClientStack;
use log::trace;

use super::TcpStream;
use crate::message::{Message, PayloadReader};

pub type IncomingMessage<'a, S, H> = Message<TcpStream<'a, S>, H>;

type OutgoingMessage<R> = Message<R, Headers>;

/// A connection between Cyberpixie peers.
pub struct Connection<S>
where
    S: TcpClientStack,
{
    socket: S::TcpSocket,

    packet_header_buf: heapless::Vec<u8, { Packet::PACKED_LEN }>,
}

impl<S> Connection<S>
where
    S: TcpClientStack,
{
    /// Creates a new connection on top of the given socket.
    pub(crate) fn new(socket: S::TcpSocket) -> Self {
        Self {
            socket,
            packet_header_buf: heapless::Vec::default(),
        }
    }

    pub fn poll_next_request<'a>(
        &'a mut self,
        stack: &'a mut S,
    ) -> nb::Result<IncomingMessage<'_, S, RequestHeader>, cyberpixie_core::Error> {
        self.poll_next_message(stack)
    }

    pub fn poll_next_response<'a>(
        &'a mut self,
        stack: &'a mut S,
    ) -> nb::Result<IncomingMessage<'_, S, ResponseHeader>, cyberpixie_core::Error> {
        self.poll_next_message(stack)
    }

    pub fn send_message(
        &mut self,
        stack: &mut S,
        header: impl Into<Headers>,
    ) -> cyberpixie_core::Result<()> {
        let header = header.into();
        trace!("Sending message {header:?}");

        let stream = TcpStream::new(stack, &mut self.socket);
        let message = OutgoingMessage::<&[u8]> {
            header,
            payload: None,
        };
        message.send_blocking(stream)
    }

    pub fn send_message_with_payload<T, P, I>(
        &mut self,
        stack: &mut S,
        header: I,
        payload: P,
    ) -> cyberpixie_core::Result<()>
    where
        T: Read,
        P: Into<PayloadReader<T>>,
        I: Into<Headers>,
    {
        let header = header.into();
        let payload = payload.into();

        trace!("Sending message {header:?} with payload {}", payload.len());

        let stream = TcpStream::new(stack, &mut self.socket);
        let message = OutgoingMessage {
            header,
            payload: Some(payload),
        };
        message.send_blocking(stream)
    }

    fn poll_next_message<'a, H>(
        &'a mut self,
        stack: &'a mut S,
    ) -> nb::Result<IncomingMessage<'a, S, H>, cyberpixie_core::Error>
    where
        H: FromPacket + Debug,
    {
        let packet = self.poll_next_packet(stack)?;

        let mut stream = TcpStream::new(stack, &mut self.socket);
        let (header, payload_len): (H, _) = packet
            .header(&mut stream)
            .map_err(cyberpixie_core::Error::network)?;

        trace!("Got a next message {header:?}, {payload_len:?}",);

        let payload = if payload_len > 0 {
            Some(PayloadReader::new(stream, payload_len))
        } else {
            None
        };

        Ok(Message { header, payload })
    }

    fn poll_next_packet(&mut self, stack: &mut S) -> nb::Result<Packet, cyberpixie_core::Error> {
        let mut buf = [0_u8; Packet::PACKED_LEN];

        let bytes_remaining = Packet::PACKED_LEN - self.packet_header_buf.len();
        let bytes_read = stack
            .receive(&mut self.socket, &mut buf[0..bytes_remaining])
            .map_err(|err| err.map(cyberpixie_core::Error::network))?;

        self.packet_header_buf
            .extend_from_slice(&buf[..bytes_read])
            .unwrap();

        if self.packet_header_buf.is_full() {
            let mut buf: &[u8] = &self.packet_header_buf;
            let packet = Packet::read(&mut buf).map_err(cyberpixie_core::Error::decode)?;
            trace!("Got a next packet {packet:?}");

            self.packet_header_buf.clear();
            Ok(packet)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

#[cfg(test)]
mod tests {
    use cyberpixie_core::proto::{
        types::{DeviceInfo, DeviceRole, PeerInfo},
        RequestHeader,
    };
    use embedded_io::blocking::Read;
    use embedded_nal::{TcpClientStack, TcpFullStack};
    use nb_utils::NbResultExt;
    use std_embedded_nal::Stack;

    type Connection = super::Connection<Stack>;

    fn create_loopback(stack: &mut Stack, port: u16) -> (Connection, Connection) {
        let mut listener = stack.socket().unwrap();
        stack.bind(&mut listener, port).unwrap();
        stack.listen(&mut listener).unwrap();

        let addr = embedded_nal::SocketAddr::from((embedded_nal::Ipv6Addr::localhost(), port));
        // Connect between two sockets.
        let mut sender = stack.socket().unwrap();
        stack.connect(&mut sender, addr).unwrap();
        let listener = nb::block!(stack.accept(&mut listener)).unwrap().0;

        (Connection::new(sender), Connection::new(listener))
    }

    #[test]
    fn test_read_write_without_payload() {
        let mut stack = Stack;

        let (mut sender, mut receiver) = create_loopback(&mut stack, 13280);

        assert!(receiver.poll_next_packet(&mut stack).is_would_block());

        let message = RequestHeader::Handshake(PeerInfo {
            role: DeviceRole::Client,
            group_id: None,
            device_info: Some(DeviceInfo::empty(64)),
        });
        sender.send_message(&mut stack, message).unwrap();

        let next_message = nb::block!(receiver.poll_next_request(&mut stack)).unwrap();
        assert_eq!(next_message.header, message);
        assert!(next_message.payload.is_none());

        assert!(receiver.poll_next_packet(&mut stack).is_would_block());
    }

    #[test]
    fn test_read_write_with_payload() {
        let mut stack = Stack;

        let (mut sender, mut receiver) = create_loopback(&mut stack, 13281);

        let text = b"Hello cyberpixie".as_slice();
        sender
            .send_message_with_payload(&mut stack, RequestHeader::Debug, text)
            .unwrap();

        let next_message = nb::block!(receiver.poll_next_request(&mut stack)).unwrap();
        assert_eq!(next_message.header, RequestHeader::Debug);

        let mut payload = next_message.payload.unwrap();
        let mut text2 = vec![0_u8; payload.len()];
        payload.read_exact(&mut text2).unwrap();

        assert_eq!(text, text2);

        assert!(receiver.poll_next_packet(&mut stack).is_would_block());
    }
}
