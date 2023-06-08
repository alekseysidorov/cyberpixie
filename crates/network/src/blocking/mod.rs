//! A connection between Cybeprixie peers.

use embedded_nal::{SocketAddr, TcpClientStack, TcpFullStack};

pub use self::{
    client::Client,
    connection::{Connection, IncomingMessage},
};
use crate::{
    core::io::{BlockingRead, BlockingWrite},
    CyberpixieError, CyberpixieResult,
};

mod client;
mod connection;

/// Network error type adapter for the embedded-io traits.
pub struct NetworkError<S: TcpClientStack>(pub S::Error);

impl<S: TcpClientStack> NetworkError<S> {
    /// Creates a new Network error value.
    pub const fn new(value: S::Error) -> Self {
        Self(value)
    }

    /// Returns an underlying value.
    pub fn into_inner(self) -> S::Error {
        self.0
    }
}

impl<S: TcpClientStack> core::fmt::Debug for NetworkError<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl<S: TcpClientStack> embedded_io::Error for NetworkError<S> {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

/// A wrapper around the [`TcpClientStack::TcpSocket`] type which provides
/// a blocking [`Read`]/[`Write`] traits implementation.
///
/// This wrapper is blocking, so it can be used in the polling scenarios.
pub struct TcpStream<'a, S: TcpClientStack> {
    stack: &'a mut S,
    socket: &'a mut S::TcpSocket,
}

impl<'a, S: TcpClientStack> TcpStream<'a, S> {
    /// Creates a new stream wrapper.
    pub fn new(stack: &'a mut S, socket: &'a mut S::TcpSocket) -> Self {
        Self { stack, socket }
    }
}

impl<'a, S: TcpClientStack> embedded_io::Io for TcpStream<'a, S> {
    type Error = NetworkError<S>;
}

impl<'a, S: TcpClientStack> BlockingWrite for TcpStream<'a, S> {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, Self::Error> {
        nb::block!(self.stack.send(self.socket, buffer)).map_err(NetworkError::new)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // There is no way to flush bytes.
        Ok(())
    }
}

impl<'a, S: TcpClientStack> BlockingRead for TcpStream<'a, S> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        nb::block!(self.stack.receive(self.socket, buffer)).map_err(NetworkError::new)
    }
}

pub struct Listener<S>
where
    S: TcpFullStack,
{
    socket: S::TcpSocket,
}

impl<S> Listener<S>
where
    S: TcpFullStack,
{
    pub fn new(stack: &mut S, local_port: u16) -> CyberpixieResult<Self> {
        let mut socket = stack.socket().map_err(CyberpixieError::network)?;
        stack
            .bind(&mut socket, local_port)
            .map_err(CyberpixieError::network)?;
        stack
            .listen(&mut socket)
            .map_err(CyberpixieError::network)?;

        Ok(Self { socket })
    }

    /// Accepts a new active incoming connection.
    ///
    /// If no pending connections are available, this function will return [`nb::Error::WouldBlock`]
    pub fn accept(
        &mut self,
        stack: &mut S,
    ) -> nb::Result<(SocketAddr, Connection<S>), CyberpixieError> {
        let (socket, address) = stack
            .accept(&mut self.socket)
            .map_err(|x| x.map(CyberpixieError::network))?;

        log::info!("Accepted incoming connection with the {address}");
        Ok((address, Connection::new(socket)))
    }
}
