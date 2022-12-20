use core::fmt::Debug;
use std::{
    collections::BTreeMap,
    net::{SocketAddr, TcpListener, TcpStream},
};

use cyberpixie_core::{
    proto::{
        types::{DeviceInfo, DeviceRole, Hertz, ImageId, ImageInfo},
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

#[derive(Debug)]
pub struct NetworkPart<S> {
    listener: TcpListener,
    connections: BTreeMap<SocketAddr, Connection>,
    device: S,
}

impl<S> NetworkPart<S>
where
    S: DeviceService,
{
    pub fn new(device: S, listener: TcpListener) -> Result<Self, anyhow::Error> {
        listener.set_nonblocking(true)?;
        Ok(Self {
            device,
            listener,
            connections: BTreeMap::default(),
        })
    }

    pub fn poll(&mut self) -> nb::Result<(), std::io::Error> {
        then_ready(self.poll_next_listener(), |(stream, address)| {
            info!("Got a new connection with {}, {:?}", address, stream);
            let connection = Connection::new(stream, self.device.device_info().role);
            self.connections.insert(address, connection);
            Ok(())
        })?;

        let mut errored_connections = Vec::new();
        for (peer, connection) in &mut self.connections {
            let result = then_ready(
                Self::poll_connection(&mut self.device, peer, connection),
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
        device: &mut S,
        peer: &SocketAddr,
        connection: &mut Connection,
    ) -> nb::Result<(), std::io::Error> {
        let request = connection.poll_next_request()?;
        info!("Got message header {:?}", request.header);

        let response = Self::handle_request(device, peer, request);
        match response {
            Ok(response) => connection.send_message(response)?,
            Err(err) => connection.send_message(ResponseHeader::Error(err))?,
        };
        Ok(())
    }

    fn handle_request(
        device: &mut S,
        peer: &SocketAddr,
        request: IncomingMessage<'_, RequestHeader>,
    ) -> Result<ResponseHeader, cyberpixie_core::Error> {
        match request.header {
            RequestHeader::Handshake(info) => {
                info!("Got a handshake from {}: {:?}", peer, info);
                Ok(ResponseHeader::Handshake(device.device_info()))
            }

            RequestHeader::AddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }) => {
                let storage = device.storage();
                let config = storage.config()?;
                if config.strip_len != strip_len {
                    return Err(cyberpixie_core::Error::StripLengthMismatch);
                }

                let image_reader = request
                    .payload
                    .ok_or(cyberpixie_core::Error::ImageLengthMismatch)?;

                let image_id = storage.add_image(refresh_rate, image_reader)?;
                Ok(ResponseHeader::AddImage(image_id))
            }

            RequestHeader::ClearImages => {
                device.storage().clear_images()?;
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
    pub device_info: DeviceInfo,
}

impl Connection {
    fn send_handshake(&mut self, host_info: DeviceInfo) -> Result<DeviceInfo, std::io::Error> {
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
}

impl Client {
    pub fn connect(stream: TcpStream) -> std::io::Result<Self> {
        let mut connection = Connection::new(stream, DeviceRole::Client);

        let device_info = connection.send_handshake(DeviceInfo::client())?;
        // TODO Check compatibility

        Ok(Self {
            connection,
            device_info,
        })
    }

    pub fn debug(&mut self, msg: &str) -> std::io::Result<()> {
        self.connection.send_debug(msg)
    }

    pub fn handshake(&mut self) -> std::io::Result<DeviceInfo> {
        self.connection.send_handshake(DeviceInfo::client())
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
}
