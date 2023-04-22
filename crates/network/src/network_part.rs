use core::fmt::Debug;
use std::{
    collections::BTreeMap,
    net::{SocketAddr, TcpListener, TcpStream},
};

use cyberpixie_core::{
    proto::{
        types::{PeerInfo, DeviceRole, Hertz, ImageId, ImageInfo},
        RequestHeader, ResponseHeader,
    },
    service::{DeviceService, DeviceStorage},
};
use log::{debug, info, trace};
use nb_utils::IntoNbResult;

use crate::connection::{Connection, IncomingMessage};

fn then_ready<T, E, F>(e: nb::Result<T, E>, then: F) -> Result<(), E>
where
    F: FnOnce(T) -> Result<(), E>,
{
    match e {
        Err(nb::Error::Other(e)) => Err(e),
        Err(nb::Error::WouldBlock) => Ok(()),
        Ok(value) => then(value),
    }
}

struct DeviceState<S: DeviceService> {
    device: S,
    render_task: Option<S::ImageRender>,
    // Cached peer info in order to reduce handshake latency.
    peer_info: PeerInfo,
}

impl<S: DeviceService> DeviceState<S> {
    fn new(device: S) -> anyhow::Result<Self> {
        let peer_info = device.peer_info()?;
        Ok(Self {
            peer_info,
            device,
            render_task: None,
        })
    }
    /// Update cached peer information.
    fn update_peer_info(&mut self) -> cyberpixie_core::Result<()> {
        self.peer_info = self.device.peer_info()?;
        Ok(())
    }
}

impl<S> std::fmt::Debug for DeviceState<S>
where
    S: DeviceService + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceState")
            .field("device", &self.device)
            .field("render_task", &self.render_task.as_ref().map(|_| ()))
            .finish()
    }
}

#[derive(Debug)]
pub struct NetworkPart<S: DeviceService> {
    listener: TcpListener,
    connections: BTreeMap<SocketAddr, Connection>,
    state: DeviceState<S>,
}

impl<S> NetworkPart<S>
where
    S: DeviceService,
{
    pub fn new(device: S, listener: TcpListener) -> Result<Self, anyhow::Error> {
        info!(
            "Creating a new network for device: {:?}",
            device.peer_info()
        );

        listener.set_nonblocking(true)?;
        Ok(Self {
            state: DeviceState::new(device)?,
            listener,
            connections: BTreeMap::default(),
        })
    }

    pub fn poll(&mut self) -> nb::Result<(), std::io::Error> {
        then_ready(self.poll_next_listener(), |(stream, address)| {
            info!("Got a new connection with {}, {:?}", address, stream);
            let connection = Connection::new(stream, self.state.peer_info.role);
            self.connections.insert(address, connection);
            Ok(())
        })?;

        let mut errored_connections = Vec::new();
        for (peer, connection) in &mut self.connections {
            let result = then_ready(
                Self::poll_connection(&mut self.state, peer, connection),
                |operation| {
                    trace!("Next operation: {:?}", operation);
                    Ok(())
                },
            );

            if let Err(err) = result {
                trace!("Connection with {peer} closed by reason: {err}");
                errored_connections.push(*peer);
            }
        }

        for peer in &errored_connections {
            self.connections.remove(peer);
        }

        Ok(())
    }

    fn poll_connection(
        state: &mut DeviceState<S>,
        peer: &SocketAddr,
        connection: &mut Connection,
    ) -> nb::Result<(), std::io::Error> {
        let request = connection.poll_next_request()?;
        info!("Got message header {:?}", request.header);

        let response = Self::handle_request(state, peer, request);
        match response {
            Ok(response) => connection.send_message(response)?,
            Err(err) => connection.send_message(ResponseHeader::Error(err))?,
        };
        Ok(())
    }

    fn handle_request(
        state: &mut DeviceState<S>,
        peer: &SocketAddr,
        request: IncomingMessage<'_, RequestHeader>,
    ) -> Result<ResponseHeader, cyberpixie_core::Error> {
        match request.header {
            RequestHeader::Handshake(info) => {
                info!("Got a handshake from {}: {:?}", peer, info);
                Ok(ResponseHeader::Handshake(state.peer_info))
            }

            RequestHeader::AddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }) => {
                let storage = state.device.storage();
                let config = storage.config()?;
                if config.strip_len != strip_len {
                    return Err(cyberpixie_core::Error::StripLengthMismatch);
                }

                let image_reader = request
                    .payload
                    .ok_or(cyberpixie_core::Error::ImageLengthMismatch)?;

                let image_id = storage.add_image(refresh_rate, image_reader)?;

                state.update_peer_info()?;
                Ok(ResponseHeader::AddImage(image_id))
            }

            RequestHeader::ShowImage(image_id) => {
                // Hide currently showing image
                if let Some(render) = state.render_task.take() {
                    state.device.hide_current_image(render)?;
                }
                // Update current image index
                state.device.storage().set_current_image_id(image_id)?;
                state.render_task = Some(state.device.show_current_image()?);

                state.update_peer_info()?;
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::HideImage => {
                if let Some(render) = state.render_task.take() {
                    state.device.hide_current_image(render)?;
                }

                state.update_peer_info()?;
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::ClearImages => {
                state.device.storage().clear_images()?;

                state.update_peer_info()?;
                Ok(ResponseHeader::Empty)
            }

            RequestHeader::Debug => {
                let message = request
                    .read_payload_to_vec()
                    .map_err(cyberpixie_core::Error::network)?;
                debug!("[{}]: {:?}", peer, String::from_utf8_lossy(&message));
                Ok(ResponseHeader::Empty)
            }
        }
    }

    fn poll_next_listener(&self) -> nb::Result<(TcpStream, SocketAddr), std::io::Error> {
        self.listener.accept().into_nb_result()
    }
}

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    pub peer_info: PeerInfo,
}

impl Connection {
    fn send_handshake(&mut self, host_info: PeerInfo) -> Result<PeerInfo, std::io::Error> {
        self.send_message(RequestHeader::Handshake(host_info))?;
        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.handshake()?)
    }

    fn send_add_image(
        &mut self,
        refresh_rate: Hertz,
        strip_len: u16,
        payload: &[u8],
    ) -> std::io::Result<ImageId> {
        self.send_message_with_payload(
            RequestHeader::AddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }),
            payload,
        )?;

        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.add_image()?)
    }

    fn send_debug(&mut self, msg: &str) -> std::io::Result<()> {
        self.send_message_with_payload(RequestHeader::Debug, msg.as_bytes())?;
        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.empty()?)
    }

    fn send_clear_images(&mut self) -> std::io::Result<()> {
        self.send_message(RequestHeader::ClearImages)?;
        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.empty()?)
    }

    fn send_show_image(&mut self, image_id: ImageId) -> std::io::Result<()> {
        self.send_message(RequestHeader::ShowImage(image_id))?;
        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.empty()?)
    }

    fn send_hide_image(&mut self) -> std::io::Result<()> {
        self.send_message(RequestHeader::HideImage)?;
        let response = nb::block!(self.poll_next_response())?;
        Ok(response.header.empty()?)
    }
}

impl Client {
    pub fn connect(stream: TcpStream) -> std::io::Result<Self> {
        let mut connection = Connection::new(stream, DeviceRole::Client);

        let peer_info = connection.send_handshake(PeerInfo::client())?;
        // TODO Check compatibility

        Ok(Self {
            connection,
            peer_info,
        })
    }

    pub fn debug(&mut self, msg: &str) -> std::io::Result<()> {
        self.connection.send_debug(msg)
    }

    pub fn handshake(&mut self) -> std::io::Result<PeerInfo> {
        self.connection.send_handshake(PeerInfo::client())
    }

    pub fn add_image(
        &mut self,
        refresh_rate: Hertz,
        strip_len: u16,
        payload: &[u8],
    ) -> std::io::Result<ImageId> {
        self.connection
            .send_add_image(refresh_rate, strip_len, payload)
    }

    pub fn clear_images(&mut self) -> std::io::Result<()> {
        self.connection.send_clear_images()
    }

    pub fn show_image(&mut self, image_id: ImageId) -> std::io::Result<()> {
        self.connection.send_show_image(image_id)
    }

    pub fn hide_image(&mut self) -> std::io::Result<()> {
        self.connection.send_hide_image()
    }
}
