use cyberpixie_core::{
    io::{AsyncRead, AsyncWrite},
    proto::{
        types::{Hertz, ImageId, ImageInfo, PeerInfo},
        RequestHeader,
    },
};
use embedded_nal::SocketAddr;

use crate::{connection::Connection, CyberpixieResult, NetworkSocket};

/// Cyberpixie network async client.
pub struct Client<C> {
    connection: Connection<C>,
}

impl<C: AsyncRead + AsyncWrite> Client<C> {
    /// Establish connection with the given peer.
    pub async fn connect<'a, S, I>(socket: &'a mut S, address: I) -> CyberpixieResult<Self>
    where
        S: NetworkSocket<Connection<'a> = C>,
        I: Into<SocketAddr>,
    {
        let address = address.into();
        let socket = socket.connect(address).await?;
        Self::new(Connection::incoming(socket)).await
    }

    /// Creates a new client on top of the given connection.
    async fn new(connection: Connection<C>) -> CyberpixieResult<Self> {
        let mut client = Self { connection };
        let peer_info = client.handshake(PeerInfo::client()).await?;
        log::info!("Handshake with the {peer_info:?}");
        Ok(client)
    }

    /// Performs handshake between peers and returns the information about the connected peer.
    async fn handshake(&mut self, host_info: PeerInfo) -> CyberpixieResult<PeerInfo> {
        self.connection
            .send_message(RequestHeader::Handshake(host_info))
            .await?;
        self.connection.receive_response().await?.header.handshake()
    }

    /// Requests an actual information about the connected peer.
    pub async fn peer_info(&mut self) -> CyberpixieResult<PeerInfo> {
        self.handshake(PeerInfo::client()).await
    }

    /// Sends a new picture to the device and returns a resulting ID.
    pub async fn add_image(
        &mut self,
        refresh_rate: Hertz,
        strip_len: u16,
        picture: &[u8],
    ) -> CyberpixieResult<ImageId> {
        self.connection
            .send_message_with_payload(
                RequestHeader::AddImage(ImageInfo {
                    refresh_rate,
                    strip_len,
                }),
                picture,
            )
            .await?;

        let response = self.connection.receive_response().await?;
        response.header.add_image()
    }

    /// Sends a debug message to the device, this message will be printed in the device log.
    pub async fn debug(&mut self, msg: &str) -> CyberpixieResult<()> {
        self.connection
            .send_message_with_payload(RequestHeader::Debug, msg.as_bytes())
            .await?;

        let response = self.connection.receive_response().await?;
        response.header.empty()
    }

    /// Sends a clear images command.
    ///
    /// The whole pictures stored in the device memory will be removed.
    pub async fn clear_images(&mut self) -> CyberpixieResult<()> {
        self.connection
            .send_message(RequestHeader::ClearImages)
            .await?;

        let response = self.connection.receive_response().await?;
        response.header.empty()
    }

    /// Sends a show image with the given ID command.
    pub async fn start(&mut self, image_id: ImageId) -> CyberpixieResult<()> {
        self.connection
            .send_message(RequestHeader::ShowImage(image_id))
            .await?;

        let response = self.connection.receive_response().await?;
        response.header.empty()
    }

    /// Send stop command.
    ///
    /// This command will stop the currently showing image and turn the device into the standby mode.
    pub async fn stop(&mut self) -> CyberpixieResult<()> {
        self.connection
            .send_message(RequestHeader::HideImage)
            .await?;

        let response = self.connection.receive_response().await?;
        response.header.empty()
    }
}
