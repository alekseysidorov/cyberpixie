//! A connection between Cybeprixie peers.

use cyberpixie_core::{
    io::{AsyncRead, AsyncWrite},
    proto::{
        packet::{FromPacket, PackedSize, Packet},
        Headers, RequestHeader, ResponseHeader,
    },
};

use crate::{CyberpixieError, CyberpixieResult, Message, PayloadReader};

/// Established connection between Cyberpixie peers.
///
/// This structure provides low-level communication API between Cyberpixie peers.
pub struct Connection<T> {
    socket: T,
}

impl<T> Connection<T>
where
    T: AsyncRead + AsyncWrite,
{
    /// Creates a new incoming connection handler on the specified raw connection with the other
    /// Cyberpixie network peers.
    pub fn incoming(socket: T) -> Self {
        Self { socket }
    }

    /// Receives a next request from the connected peer.
    pub async fn receive_request(&mut self) -> CyberpixieResult<Message<&mut T, RequestHeader>> {
        self.receive_message().await
    }

    /// Receives a next response from the connected peer.
    pub async fn receive_response(&mut self) -> CyberpixieResult<Message<&mut T, ResponseHeader>> {
        self.receive_message().await
    }

    /// Sends a message without payload to the connected peer.
    pub async fn send_message(&mut self, header: impl Into<Headers>) -> CyberpixieResult<()> {
        let header = header.into();
        log::trace!("Sending message {header:?}");
        Message::new(header).send_async(&mut self.socket).await
    }

    /// Sends a message with payload to the connected peer.
    pub async fn send_message_with_payload<R, P, I>(
        &mut self,
        header: I,
        payload: P,
    ) -> cyberpixie_core::Result<()>
    where
        R: embedded_io::blocking::Read,
        P: Into<PayloadReader<R>>,
        I: Into<Headers>,
    {
        let header = header.into();
        let payload = payload.into();

        log::trace!("Sending message {header:?} with payload {}", payload.len());

        Message {
            header,
            payload: Some(payload),
        }
        .send_async(&mut self.socket)
        .await
    }

    /// Receives a next incoming message from the connected peer.
    async fn receive_message<H: FromPacket>(&mut self) -> CyberpixieResult<Message<&mut T, H>> {
        // Read packet header
        let mut buf = [0_u8; Packet::MAX_LEN];
        self.socket
            .read_exact(&mut buf[0..Packet::PACKED_LEN])
            .await
            .map_err(|_| CyberpixieError::Network)?;
        // Decode it
        let packet = Packet::from_bytes(&buf[0..Packet::PACKED_LEN]);
        log::trace!("Got a next packet {packet:?}");

        // Read header
        let header_len = packet.header_len as usize;
        if header_len >= Packet::MAX_LEN {
            return Err(CyberpixieError::Decode);
        }

        self.socket
            .read_exact(&mut buf[0..header_len])
            .await
            .map_err(|_| CyberpixieError::Network)?;
        let header = H::from_bytes(&buf[0..header_len]).map_err(CyberpixieError::decode)?;

        Ok(Message {
            header,
            payload: packet
                .has_payload()
                .then_some(PayloadReader::new(&mut self.socket, packet.payload_len())),
        })
    }
}
