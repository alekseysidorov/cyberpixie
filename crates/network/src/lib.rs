//! Cyberpixie Network abstraction layer
//!
//! This crate provides a implementation agnostic network layer for the cyberpixie project.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::missing_const_for_fn
)]

pub use connection::{Connection, IncomingMessage, Message};
use cyberpixie_core::{
    proto::{
        types::{Hertz, ImageId, ImageInfo, PeerInfo},
        RequestHeader,
    },
    Error as CyberpixieError, Result as CyberpixieResult,
};
pub use cyberpixie_core as core;
pub use embedded_nal::SocketAddr;
use embedded_nal::{TcpClientStack, TcpFullStack};

mod connection;
pub mod io;

/// Client connection to another Cyberpixie peer.
pub struct Client<S>
where
    S: TcpClientStack,
{
    /// Raw connection object.
    inner: connection::Connection<S>,
}

impl<S> Client<S>
where
    S: TcpClientStack,
{
    /// Establish connection with the given peer.
    pub fn connect(stack: &mut S, address: impl Into<SocketAddr>) -> CyberpixieResult<Self> {
        // Create socket and establish connection with the peer.
        let mut socket = stack.socket().map_err(CyberpixieError::network)?;
        stack
            .connect(&mut socket, address.into())
            .map_err(CyberpixieError::network)?;
        // Create a client instance.
        let mut client = Self {
            inner: Connection::new(socket),
        };
        // And then perform handshake.
        let peer_info = client.handshake(stack, PeerInfo::client())?;

        log::info!("Established connection with the {peer_info:?}");
        Ok(client)
    }

    /// Requests an actual information about the connected peer.
    pub fn peer_info(&mut self, stack: &mut S) -> CyberpixieResult<PeerInfo> {
        self.handshake(stack, PeerInfo::client())
    }

    /// Sends a new picture to the device and returns a resulting ID.
    pub fn add_image(
        &mut self,
        stack: &mut S,
        refresh_rate: Hertz,
        strip_len: u16,
        picture: &[u8],
    ) -> CyberpixieResult<ImageId> {
        self.inner.send_message_with_payload(
            stack,
            RequestHeader::AddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }),
            picture,
        )?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.add_image()
    }

    /// Sends a debug message to the device, this message will be printed in the device log.
    pub fn debug(&mut self, stack: &mut S, msg: &str) -> CyberpixieResult<()> {
        self.inner
            .send_message_with_payload(stack, RequestHeader::Debug, msg.as_bytes())?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.empty()
    }

    /// Sends a clear images command.
    ///
    /// The whole pictures stored in the device memory will be removed.
    pub fn clear_images(&mut self, stack: &mut S) -> CyberpixieResult<()> {
        self.inner.send_message(stack, RequestHeader::ClearImages)?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.empty()
    }

    /// Sends a show image with the given ID command.
    pub fn start(&mut self, stack: &mut S, image_id: ImageId) -> CyberpixieResult<()> {
        self.inner
            .send_message(stack, RequestHeader::ShowImage(image_id))?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.empty()
    }

    /// Send stop command.
    ///
    /// This command will stop the currently showing image and turn the device into the standby mode.
    pub fn stop(&mut self, stack: &mut S) -> CyberpixieResult<()> {
        self.inner.send_message(stack, RequestHeader::HideImage)?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.empty()
    }

    /// Performs handshake between peers and returns the information about the connected peer.
    fn handshake(&mut self, stack: &mut S, host_info: PeerInfo) -> CyberpixieResult<PeerInfo> {
        let message = RequestHeader::Handshake(host_info);
        self.inner.send_message(stack, message)?;

        let response = nb::block!(self.inner.poll_next_response(stack))?;
        response.header.handshake()
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
