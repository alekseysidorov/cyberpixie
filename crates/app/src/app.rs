//! Cybeprixie application business-logic implementation

use cyberpixie_core::{
    proto::{
        types::{DeviceInfo, DeviceRole, ImageInfo, PeerInfo},
        RequestHeader, ResponseHeader,
    },
    ExactSizeRead,
};
use cyberpixie_network::{Connection, Listener, SocketAddr};
use embedded_io::blocking::Read;
use nb_utils::NbResultExt;

use crate::{Board, CyberpixieError, CyberpixieResult, Storage, NETWORK_PORT};

/// Max incoming connections amount.
const MAX_CONNECTIONS: usize = 4;

/// Incoming connections map.
type Connections<S> = heapless::LinearMap<SocketAddr, Connection<S>, MAX_CONNECTIONS>;

/// Cybeprixie application runner
pub struct App<B: Board> {
    board: B,
    network: B::NetworkStack,
    listener: Listener<B::NetworkStack>,

    storage: Option<B::Storage>,
    render: Option<B::RenderTask>,
    // Cached device information.
    device_info: DeviceInfo,
}

impl<B: Board> App<B> {
    /// Creates a new application instance for the given board on a specified network port.
    pub fn with_port(mut board: B, port: u16) -> CyberpixieResult<Self> {
        let (storage, mut network) = board
            .take_components()
            .expect("Board components has been already taken");

        let listener = Listener::new(&mut network, port)?;
        let device_info = read_device_info(&storage)?;

        Ok(Self {
            board,
            network,
            listener,

            storage: Some(storage),
            render: None,
            device_info,
        })
    }

    /// Creates a new application instance for the given board.
    pub fn new(board: B) -> CyberpixieResult<Self> {
        Self::with_port(board, NETWORK_PORT)
    }

    /// Runs an Cyberpixie application
    pub fn run(mut self) -> CyberpixieResult<()> {
        let mut connections = Connections::default();
        // Start event loop.
        loop {
            self.poll_network(&mut connections)?;
        }
    }

    /// Polls a next network event.
    fn poll_network(
        &mut self,
        connections: &mut Connections<B::NetworkStack>,
    ) -> CyberpixieResult<()> {
        // Check for the incoming connections.
        self.listener
            .accept(&mut self.network)
            .if_ready(|(address, connection)| {
                connections.insert(address, connection).map_err(|_| {
                    CyberpixieError::internal(format!(
                        "Too many peers, the device cannot handle more than the {} peers",
                        MAX_CONNECTIONS
                    ))
                })?;
                Ok(())
            })?;
        // Poll the entire incoming connections.
        let mut errored_connections: heapless::Vec<_, MAX_CONNECTIONS> = heapless::Vec::new();
        for (address, peer) in connections.iter_mut() {
            // Poll peer events.
            let result = self.poll_peer(*address, peer);
            if let Err(error) = result {
                log::info!("Connection with {address} closed by reason: {error}");
                errored_connections.push(*address).unwrap();
            }
        }
        // Close the errored connections.
        for peer in errored_connections {
            connections.remove(&peer);
        }
        Ok(())
    }

    /// Returns a storage reference.
    fn storage(&self) -> CyberpixieResult<&B::Storage> {
        // TODO Handle this situation more properly.
        self.storage.as_ref().ok_or(CyberpixieError::Internal)
    }

    /// Returns a mutable storage reference.
    fn storage_mut(storage: &mut Option<B::Storage>) -> CyberpixieResult<&mut B::Storage> {
        // TODO Handle this situation more properly.
        storage.as_mut().ok_or(CyberpixieError::Internal)
    }

    /// Returns a mutable storage reference.
    ///
    /// If an image rendering task is being active, then it is interrupted it to get back
    /// a storage instance.
    fn stop_rendering<'a>(
        board: &mut B,
        storage: &'a mut Option<B::Storage>,
        render: &mut Option<B::RenderTask>,
    ) -> CyberpixieResult<&'a mut B::Storage> {
        if let Some(handle) = render.take() {
            storage.replace(board.stop_rendering(handle)?);
        }

        Ok(storage.as_mut().unwrap())
    }

    /// Handles network requests from the given peer.
    fn poll_peer(
        &mut self,
        address: SocketAddr,
        peer: &mut Connection<B::NetworkStack>,
    ) -> CyberpixieResult<()> {
        let request = match peer.poll_next_request(&mut self.network) {
            Ok(request) => request,
            Err(nb::Error::WouldBlock) => return Ok(()),
            Err(nb::Error::Other(err)) => return Err(err),
        };

        // Handle incoming request.
        let response = match request.header {
            RequestHeader::Handshake(info) => {
                log::info!("Got a handshake from {}: {:?}", address, info);
                Ok(ResponseHeader::Handshake(self.peer_info()))
            }

            RequestHeader::Debug => {
                // FIXME find the way to put message to debug log level instead of eprintln
                // and support for unicode
                if let Some(mut payload) = request.payload {
                    while payload.bytes_remaining() != 0 {
                        let mut byte = [0_u8];
                        payload.read(&mut byte).map_err(CyberpixieError::network)?;
                        eprint!("{}", byte[0] as char);
                    }
                    eprintln!();
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

                let image = request
                    .payload
                    .ok_or(CyberpixieError::ImageLengthMismatch)?;

                let image_id =
                    Self::storage_mut(&mut self.storage)?.add_image(refresh_rate, image)?;

                // Since we change the number of images we have to refresh device information.
                self.refresh_device_info()?;
                Ok(ResponseHeader::AddImage(image_id))
            }

            RequestHeader::ShowImage(image_id) => {
                let storage =
                    Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)?;
                storage.set_current_image_id(image_id)?;

                let render = self
                    .board
                    .start_rendering(self.storage.take().unwrap(), image_id)?;
                self.render = Some(render);
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::HideImage => {
                Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)?;
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::ClearImages => {
                let storage =
                    Self::stop_rendering(&mut self.board, &mut self.storage, &mut self.render)?;
                storage.clear_images()?;
                // Since we change the number of images we have to refresh device information.
                self.refresh_device_info()?;
                Ok(ResponseHeader::Empty)
            }
        };

        // Send response for the incoming request.
        match response {
            Ok(response) => peer.send_message(&mut self.network, response)?,
            Err(err) => peer.send_message(&mut self.network, ResponseHeader::Error(err))?,
        };
        Ok(())
    }

    /// Refreshes cached device information
    fn refresh_device_info(&mut self) -> CyberpixieResult<()> {
        let storage = self.storage()?;
        self.device_info = read_device_info(storage)?;
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
}

fn read_device_info<S: Storage>(storage: &S) -> CyberpixieResult<DeviceInfo> {
    let config = storage.config()?;
    Ok(DeviceInfo {
        strip_len: config.strip_len,
        images_count: storage.images_count()?,
        current_image: config.current_image,
        active: false,
    })
}
