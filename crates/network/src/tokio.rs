//! Network stack implementation for the Tokio types.

use std::net::{Ipv6Addr, SocketAddr};

// use embedded_io_adapters::tokio_1::FromTokio;
use embedded_io::adapters::FromTokio;
use tokio::net::{TcpListener, TcpStream};

use super::{NetworkSocket, NetworkStack};
use crate::{
    core::io::{AsyncRead, AsyncWrite, ErrorType},
    CyberpixieError, CyberpixieResult,
};

/// The [`tokio`] based Cyberpixie network stack.
#[derive(Default)]
pub struct TokioStack;

/// Ephemeral socket type.
pub struct TokioSocket;

/// Type holding a TCP connection state.
pub struct TokioConnection {
    /// TCP stream itself.
    stream: FromTokio<TcpStream>,
    /// Hold the listener instance if this connection is incoming.
    _listener: Option<TcpListener>,
}

impl ErrorType for TokioConnection {
    type Error = std::io::Error;
}

impl AsyncRead for TokioConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.stream.read(buf).await
    }
}

impl AsyncWrite for TokioConnection {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.stream.write(buf).await
    }
}

impl NetworkSocket for TokioSocket {
    type ConnectionError = std::io::Error;
    type Connection<'a> = TokioConnection;

    async fn accept(&mut self, port: u16) -> CyberpixieResult<Self::Connection<'_>> {
        // Create listener
        let local_address = SocketAddr::from((Ipv6Addr::LOCALHOST, port));
        let listener = TcpListener::bind(local_address)
            .await
            .map_err(CyberpixieError::network)?;
        log::info!("Bound listener on the {local_address}");
        // Accept the first incoming connection.
        let (stream, address) = listener.accept().await.map_err(CyberpixieError::network)?;
        log::info!("Accepted an incoming connection from the {address}");
        Ok(TokioConnection {
            stream: FromTokio::new(stream),
            _listener: Some(listener),
        })
    }

    async fn connect(&mut self, addr: SocketAddr) -> CyberpixieResult<Self::Connection<'_>> {
        // Just convert a socket address type by using the string representation.
        let addr: std::net::SocketAddr = addr.to_string().parse().unwrap();
        let stream = TcpStream::connect(addr)
            .await
            .map_err(CyberpixieError::network)?;

        Ok(TokioConnection {
            stream: FromTokio::new(stream),
            _listener: None,
        })
    }
}

impl NetworkStack for TokioStack {
    type Socket<'a> = TokioSocket;

    fn socket(&mut self) -> Self::Socket<'_> {
        TokioSocket
    }
}
