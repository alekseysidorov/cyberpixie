//! Cyberpixie Network abstraction layer
//!
//! This crate provides a implementation agnostic network layer for the cyberpixie project.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Features
#![feature(async_fn_in_trait)]
// Linter configuration
#![warn(unsafe_code, clippy::pedantic, clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::missing_const_for_fn
)]

pub use cyberpixie_core as core;
use cyberpixie_core::{
    io::{AsyncRead, AsyncWrite},
    Error as CyberpixieError, Result as CyberpixieResult,
};
pub use no_std_net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};

pub use crate::{
    client::Client,
    connection::Connection,
    message::{Message, PayloadReader},
};

mod client;
mod connection;
mod message;

/// Default IP address of the device.
pub const DEFAULT_DEVICE_IP_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

#[cfg(feature = "tokio")]
pub mod tokio;

/// The trait allows to create a certain TCP sockets which can do the following operations:
///
/// - Accept an incoming connection on the given port.
/// - Connect to the given address.
///
/// Regardless of the connection method each socket implements an I/O traits from the
/// [`embedded-io`] project.
pub trait NetworkStack {
    /// Type provides a network socket operations.
    type Socket<'a>: NetworkSocket
    where
        Self: 'a;
    /// Creates a new network socket.
    ///
    /// The socket must be connected before it can be used.
    fn socket(&mut self) -> Self::Socket<'_>;
}

/// Trait provides a common operations with the network socket.
pub trait NetworkSocket {
    /// Error type returned on connection failure.
    type ConnectionError: embedded_io::Error;
    /// Type holding of a TCP connection state. Should close the connection when dropped.
    type Connection<'a>: AsyncRead<Error = Self::ConnectionError>
        + AsyncWrite<Error = Self::ConnectionError>;
    /// Accepts an active incoming connection on the specified local port
    ///
    /// Returns `Ok(connection)` when a new pending connection was created.
    async fn accept(&mut self, port: u16) -> CyberpixieResult<Self::Connection<'_>>;
    /// Connects to a remote peer with the given address.
    ///
    /// Returns `Ok(connection)` when a connection was established.
    async fn connect(&mut self, addr: SocketAddr) -> CyberpixieResult<Self::Connection<'_>>;
}

/// The trait used to socket address conversion into the network stack specific type.
pub trait FromSocketAddress {
    /// Converts to this type from an input socket address.
    fn from_socket_address(value: SocketAddr) -> Self;
}

#[cfg(feature = "std")]
impl FromSocketAddress for std::net::SocketAddr {
    fn from_socket_address(value: SocketAddr) -> Self {
        value
    }
}

#[cfg(feature = "embassy-net")]
impl FromSocketAddress for smoltcp::wire::IpEndpoint {
    fn from_socket_address(value: SocketAddr) -> Self {
        let (addr, port) = FromSocketAddress::from_socket_address(value);
        Self { addr, port }
    }
}

#[cfg(feature = "embassy-net")]
impl FromSocketAddress for (smoltcp::wire::IpAddress, u16) {
    fn from_socket_address(value: SocketAddr) -> Self {
        match value {
            SocketAddr::V4(socket) => {
                let ip = smoltcp::wire::Ipv4Address(socket.ip().octets()).into_address();
                let port = socket.port();
                (ip, port)
            }
            SocketAddr::V6(socket) => {
                let ip = smoltcp::wire::Ipv6Address(socket.ip().octets()).into_address();
                let port = socket.port();
                (ip, port)
            }
        }
    }
}
