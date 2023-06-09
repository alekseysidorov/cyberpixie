//! Cybeprixie application business-logic implementation

use cyberpixie_core::{
    proto::{
        types::{DeviceInfo, DeviceRole, ImageInfo, PeerInfo},
        RequestHeader, ResponseHeader,
    },
    ExactSizeRead, BYTES_PER_PIXEL,
};
use cyberpixie_network::{Connection, Message, NetworkSocket, NetworkStack};

use super::{Board, DEFAULT_CLIENT_PORT};
use crate::{CyberpixieError, CyberpixieResult, Storage};

/// Cyberpixie application runner.
pub struct App<B: Board> {
    port: u16,
    network: B::NetworkStack,
    inner: AppInner<B>,
}

impl<B: Board> App<B> {
    /// Creates a new application instance.
    pub fn new(board: B) -> CyberpixieResult<Self> {
        Self::with_port(board, DEFAULT_CLIENT_PORT)
    }

    /// Creates a new application instance that listens client requests on the specified port.
    pub fn with_port(mut board: B, port: u16) -> CyberpixieResult<Self> {
        let (mut storage, network) = board
            .take_components()
            .expect("Board components has been already taken");

        let device_info = crate::read_device_info(&mut storage)?;
        Ok(Self {
            network,
            port,
            inner: AppInner {
                board,
                storage: Some(storage),
                render: None,
                device_info,
            },
        })
    }

    /// Runs a Cyberpixie application event loop.
    pub async fn run(mut self) -> CyberpixieResult<()> {
        loop {
            if let Err(_err) = self.run_client_requests_handler().await {
                log::info!("Closed connection with client");
            }
        }
    }

    async fn run_client_requests_handler(&mut self) -> CyberpixieResult<()> {
        let mut client_socket = self.network.socket();
        // Wait for a new incoming Client connection
        let mut peer = Connection::incoming(client_socket.accept(self.port).await?);
        // Run client requests handler.
        loop {
            let mut request = peer.receive_request().await?;
            let response = self
                .inner
                .handle_client_request(&mut request)
                .await
                .unwrap_or_else(ResponseHeader::Error);

            // It the payload has not been read by the handler, we must read it anyway
            // in order to avoid malformed socked state.
            if let Some(payload) = request.payload.take() {
                payload.skip().await.map_err(CyberpixieError::network)?;
            }

            peer.send_message(response).await?;
        }
    }
}

struct AppInner<B: Board> {
    board: B,

    storage: Option<B::Storage>,
    render: Option<B::RenderTask>,
    // Cached device information.
    device_info: DeviceInfo,
}

impl<B: Board> AppInner<B> {
    /// Returns a storage reference.
    fn storage_mut(storage: &mut Option<B::Storage>) -> CyberpixieResult<&mut B::Storage> {
        // TODO Handle this situation more properly.
        storage.as_mut().ok_or(CyberpixieError::Internal)
    }

    /// Refreshes cached device information
    fn refresh_device_info(&mut self) -> CyberpixieResult<()> {
        let storage = Self::storage_mut(&mut self.storage)?;
        self.device_info = crate::read_device_info(storage)?;
        Ok(())
    }

    /// Returns peer information about this running application for handshake.
    fn peer_info(&mut self) -> PeerInfo {
        PeerInfo {
            role: DeviceRole::Main,
            group_id: None,
            device_info: Some(DeviceInfo {
                active: self.render.is_some(),
                ..self.device_info
            }),
        }
    }

    /// Stops rendering and returns a mutable storage reference.
    ///
    /// If an image rendering task is being active, then it is interrupted it to get back
    /// a storage instance.
    async fn stop_rendering<'a>(
        board: &mut B,
        storage: &'a mut Option<B::Storage>,
        render: &mut Option<B::RenderTask>,
    ) -> CyberpixieResult<&'a mut B::Storage> {
        if let Some(handle) = render.take() {
            storage.replace(board.stop_rendering(handle).await?);
        }

        Ok(storage.as_mut().unwrap())
    }

    /// Handles incoming client request
    async fn handle_client_request<R: embedded_io::asynch::Read>(
        &mut self,
        request: &mut Message<R, RequestHeader>,
    ) -> CyberpixieResult<ResponseHeader> {
        match request.header {
            RequestHeader::Handshake(info) => {
                log::info!("Got a handshake with: {:?}", info);
                Ok(ResponseHeader::Handshake(self.peer_info()))
            }

            RequestHeader::Debug => {
                if let Some(payload) = request.payload.take() {
                    self.board.show_debug_message(payload).await?;
                }
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::AddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }) => {
                if self.device_info.strip_len != strip_len {
                    return Err(CyberpixieError::StripLengthMismatch);
                }
                // Request should has payload.
                let image = request
                    .payload
                    .take()
                    .ok_or(CyberpixieError::ImageLengthMismatch)?;
                // The length of the picture in bytes should be a multiple of "strip length" * "bytes per pixel".
                if image.bytes_remaining() % (strip_len as usize * BYTES_PER_PIXEL) != 0 {
                    // Don't forget to skip the entire payload.
                    image.skip().await.map_err(CyberpixieError::network)?;
                    return Err(CyberpixieError::ImageLengthMismatch);
                }

                let storage =
                    Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)
                        .await?;
                let image_id = storage.add_image(refresh_rate, image).await?;

                // Since we change the number of images we have to refresh device information.
                self.refresh_device_info()?;
                Ok(ResponseHeader::AddImage(image_id))
            }

            RequestHeader::ShowImage(image_id) => {
                let storage =
                    Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)
                        .await?;
                storage.set_current_image_id(image_id)?;
                // Since we change the current image ID we have to refresh device information.
                self.refresh_device_info()?;

                let render = self
                    .board
                    .start_rendering(self.storage.take().unwrap(), image_id)
                    .await?;
                self.render = Some(render);
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::HideImage => {
                Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render).await?;
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::ClearImages => {
                let storage =
                    Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)
                        .await?;
                storage.clear_images()?;
                // Since we change the number of images we have to refresh device information.
                self.refresh_device_info()?;
                Ok(ResponseHeader::Empty)
            }
        }
    }
}
