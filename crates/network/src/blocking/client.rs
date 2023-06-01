//! A cyberpixie client implementation

use cyberpixie_core::proto::{
    types::{Hertz, ImageId, ImageInfo, PeerInfo},
    RequestHeader,
};
use embedded_nal::{SocketAddr, TcpClientStack};

use super::Connection;
use crate::{CyberpixieError, CyberpixieResult};

/// Client connection to another Cyberpixie peer.
pub struct Client<S>
where
    S: TcpClientStack,
{
    /// Raw connection object.
    inner: Connection<S>,
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
