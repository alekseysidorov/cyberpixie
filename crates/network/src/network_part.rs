use core::fmt::Debug;
use std::{
    collections::BTreeMap,
    io::ErrorKind,
    net::{SocketAddr, TcpListener, TcpStream},
};

use cyberpixie_core::{DeviceService, DeviceStorage};
use cyberpixie_proto::{
    types::{DeviceInfo, DeviceRole, Hertz, ImageId, ImageInfo},
    MessageHeader,
};
use log::{debug, info, trace};
use nb_utils::IntoNbResult;

use crate::connection::Connection;

fn other_io_error(err: impl Debug) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, format!("{err:?}"))
}

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

#[derive(Debug, Default)]
enum Operation {
    #[default]
    None,
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
    ) -> nb::Result<Operation, std::io::Error> {
        let next_message = connection.poll_next_message()?;
        info!("Got message header {:?}", next_message.header);
        let next_operation = match next_message.header {
            MessageHeader::RequestHandshake(info) => {
                info!("Got a handshake from {}: {:?}", peer, info);
                let info = device.device_info();
                connection.respond_handshake(info)?;
                Operation::None
            }

            MessageHeader::Debug => {
                let message = next_message.read_payload_to_vec()?;
                debug!("[{}]: {:?}", peer, String::from_utf8_lossy(&message));
                connection.respond_ok()?;
                Operation::None
            }
            MessageHeader::RequestAddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }) => {
                let storage = device.storage();
                // TODO check strip len
                let image = next_message.payload.unwrap();

                assert_eq!(
                    storage
                        .config()
                        .map_err(|_| other_io_error("Unable to get config"))?
                        .strip_len,
                    strip_len,
                    "wrong strip lenght"
                );

                let image_id = storage
                    .add_image(refresh_rate, image)
                    .map_err(|_| other_io_error("Unable to add image"))?;

                connection.respond_add_image(image_id)?;
                Operation::None
            }

            MessageHeader::RequestClearImages => {
                device
                    .storage()
                    .clear_images()
                    .map_err(|_| other_io_error("Unable to add image"))?;
                connection.respond_ok()?;
                Operation::None
            }

            other => unreachable!("{other:?}"),
        };
        Ok(next_operation)
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
        // TODO create macro to get rid of boiler-plate.
        let header = MessageHeader::RequestHandshake(host_info);
        self.send_message(header)?;
        // Get response
        let response = nb::block!(self.poll_next_message())?;
        assert!(response.payload.is_none());

        if let MessageHeader::ResponseHandshake(handshake) = response.header {
            Ok(handshake)
        } else {
            Err(std::io::Error::new(
                ErrorKind::Other,
                "Got an unexpected response",
            ))
        }
    }

    fn send_add_image(
        &mut self,
        refresh_rate: Hertz,
        strip_len: u16,
        payload: &[u8],
    ) -> std::io::Result<ImageId> {
        self.send_message_with_payload(
            MessageHeader::RequestAddImage(ImageInfo {
                refresh_rate,
                strip_len,
            }),
            payload,
        )?;

        // Get response
        let response = nb::block!(self.poll_next_message())?;
        assert!(response.payload.is_none());

        if let MessageHeader::ResponseAddImage(id) = response.header {
            Ok(id)
        } else {
            Err(std::io::Error::new(
                ErrorKind::Other,
                "Got an unexpected response",
            ))
        }
    }

    fn send_debug(&mut self, msg: &str) -> std::io::Result<()> {
        self.send_message_with_payload(MessageHeader::Debug, msg.as_bytes())?;
        // Get response
        self.wait_for_ok()
    }

    fn send_clear_images(&mut self) -> std::io::Result<()> {
        self.send_message(MessageHeader::RequestClearImages)?;
        // Get response
        self.wait_for_ok()
    }

    fn wait_for_ok(&mut self) -> std::io::Result<()> {
        let response = nb::block!(self.poll_next_message())?;
        assert!(response.payload.is_none());

        if let MessageHeader::ResponseOk = response.header {
            Ok(())
        } else {
            Err(std::io::Error::new(
                ErrorKind::Other,
                "Got an unexpected response",
            ))
        }
    }

    fn respond_handshake(&mut self, host_info: DeviceInfo) -> Result<(), std::io::Error> {
        self.send_message(MessageHeader::ResponseHandshake(host_info))
    }

    fn respond_ok(&mut self) -> Result<(), std::io::Error> {
        self.send_message(MessageHeader::ResponseOk)
    }

    fn respond_add_image(&mut self, image_id: ImageId) -> Result<(), std::io::Error> {
        self.send_message(MessageHeader::ResponseAddImage(image_id))
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
