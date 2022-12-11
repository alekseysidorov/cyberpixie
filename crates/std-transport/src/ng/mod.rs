use std::{
    collections::BTreeMap,
    io::ErrorKind,
    net::{SocketAddr, TcpListener, TcpStream},
};

use cyberpixie_proto::ng::{DeviceInfo, DeviceRole, MessageHeader};
use log::{debug, info, trace};
use nb_utils::IntoNbResult;

use self::connection::Connection;

mod connection;

macro_rules! then_ready {
    ($e:expr, $value:pat => $then:expr) => {
        match $e {
            Err(nb::Error::Other(e)) => Err(e),
            Err(nb::Error::WouldBlock) => Ok(()),
            Ok($value) => {
                $then
                Ok(())
            },
        }
    };
}

pub trait SimpleDevice {
    fn device_info(&self) -> DeviceInfo;
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

impl<S: SimpleDevice> NetworkPart<S> {
    pub fn new(device: S, listener: TcpListener) -> Result<Self, anyhow::Error> {
        listener.set_nonblocking(true)?;
        Ok(Self {
            device,
            listener,
            connections: BTreeMap::default(),
        })
    }

    pub fn poll(&mut self) -> nb::Result<(), std::io::Error> {
        then_ready!(
            self.poll_next_listener(),
            (stream, address) => {
                info!("Got a new connection with {}, {:?}", address, stream);
                let connection = Connection::new(stream, self.device.device_info().role);
                self.connections.insert(address, connection);
            }
        )?;

        let mut errored_connections = Vec::new();
        for (peer, connection) in &mut self.connections {
            let result = then_ready!(
                Self::poll_connection(&mut self.device, peer, connection),
                operation => {
                    trace!("Next operation: {:?}", operation);
                }
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
                Operation::None
            }
            MessageHeader::RequestAddImage(_) => todo!(),

            MessageHeader::ResponseHandshake(msg) => unreachable!("{:?}", msg),
            MessageHeader::ResponseAddImage(msg) => unreachable!("{:?}", msg),
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

    fn respond_handshake(&mut self, host_info: DeviceInfo) -> Result<(), std::io::Error> {
        self.send_message(MessageHeader::ResponseHandshake(host_info))
    }
}

impl Client {
    pub fn connect(stream: TcpStream) -> std::io::Result<Self> {
        let mut connection = Connection::new(stream, DeviceRole::Client);

        connection.send_message_with_payload(
            MessageHeader::Debug,
            "Test message with payload".as_bytes(),
        )?;

        let device_info = connection.send_handshake(DeviceInfo::client())?;
        // TODO Check compatibility

        Ok(Self {
            connection,
            device_info,
        })
    }

    pub fn send_debug(&mut self, msg: &str) -> std::io::Result<()> {
        self.connection
            .send_message_with_payload(MessageHeader::Debug, msg.as_bytes())
    }

    pub fn resend_handshake(&mut self) -> std::io::Result<DeviceInfo> {
        self.connection.send_handshake(DeviceInfo::client())
    }
}
