//! An async version of the network abstraction layer

use cyberpixie_core::proto::{
    packet::{FromPacket, PackedSize, Packet},
    Headers, RequestHeader, ResponseHeader,
};
use embedded_io::asynch::{Read, Write};
use embedded_nal::SocketAddr;

use crate::{CyberpixieError, CyberpixieResult, Message, PayloadReader};

pub mod client;

/// The trait allows the underlying driver to listen a certain socket address and accepts an
/// incoming connections that implement the I/O traits from the [`embedded-io`] project.
///
/// The associated connection type should close the connection when dropped.
pub trait TcpListener {
    /// Error type returned on connection failure.
    type Error: embedded_io::Error;
    /// Type holding of a TCP connection state. Should close the connection when dropped.
    type Connection<'a>: Read<Error = Self::Error> + Write<Error = Self::Error>
    where
        Self: 'a;

    /// Binds listener to the specified local port.
    ///
    /// Returns `Ok` when a listener is successfully bound to the specified local port.
    /// Otherwise returns an `Err(e)` variant.
    async fn bind(&mut self, port: u16) -> Result<(), Self::Error>;
    /// Accepts an active incoming connection
    ///
    /// Returns `Ok(connection)` when a new pending connection was created.
    async fn accept(&mut self) -> Result<(SocketAddr, Self::Connection<'_>), Self::Error>;
}

/// An incoming Cybeprixie connections listener.
///
/// This listener handles an incoming connections from the other Cyberpixie network
/// peers and receives messages from them. It uses an abstract [`TcpListener`] under the hood.
pub struct Listener<T> {
    listener: T,
}

impl<T> Listener<T> {
    /// Creates a new Cybeprixie network listener.
    pub fn new(listener: T) -> Self {
        Self { listener }
    }
}

impl<T: TcpListener> Listener<T> {
    /// Accepts an active incoming connection.
    pub async fn accept(&mut self) -> CyberpixieResult<Connection<T::Connection<'_>>> {
        let (address, socket) = self
            .listener
            .accept()
            .await
            .map_err(|_| CyberpixieError::Network)?;
        log::info!("Accepted incoming connection with the {address}");
        Ok(Connection { socket })
    }
}

/// Established connection between Cyberpixie peers.
///
/// This structure provides low-level communication API between Cyberpixie peers.
pub struct Connection<T> {
    socket: T,
}

impl<T> Connection<T>
where
    T: Read + Write,
{
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
