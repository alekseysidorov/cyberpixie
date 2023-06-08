use cyberpixie_core::{
    io::{AsyncRead, AsyncWrite},
    proto::{types::PeerInfo, RequestHeader},
};

use super::Connection;
use crate::CyberpixieResult;

/// Cyberpixie network async client.
pub struct Client<S> {
    connection: Connection<S>,
}

impl<S: AsyncRead + AsyncWrite> Client<S> {
    /// Creates a new client on top of the given connection.
    pub async fn new(connection: Connection<S>) -> CyberpixieResult<Self> {
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
}
