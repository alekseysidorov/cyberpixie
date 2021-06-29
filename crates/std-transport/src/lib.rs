use std::{
    fmt::Display,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

use cyberpixie_proto::{
    DeviceRole, Handshake, PacketData, PacketKind, PacketWithPayload, Service, Transport,
    TransportEvent,
};

const TIMEOUT: Duration = Duration::from_secs(15);
const HOST_DEVICE_ID: [u32; 4] = [0; 4];

pub fn display_err(err: impl Display) -> anyhow::Error {
    anyhow::format_err!("{}", err)
}

pub fn connect_to(addr: &SocketAddr) -> anyhow::Result<TcpStream> {
    log::debug!("Connecting to the {}", addr);
    let stream = TcpStream::connect_timeout(addr, TIMEOUT)?;
    log::debug!("Connected");

    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.set_nodelay(true).ok();

    Ok(stream)
}

pub fn create_service(addr: SocketAddr) -> anyhow::Result<Service<TcpTransport>> {
    let stream = connect_to(&addr)?;
    let transport = TcpTransport::new(addr, stream);

    let mut service = Service::new(transport, 512);
    let response = service
        .handshake(
            addr,
            Handshake {
                device_id: HOST_DEVICE_ID,
                group_id: None,
                role: DeviceRole::Host,
            },
        )?
        .map_err(display_err)?;
    log::trace!("Connected with device: {:?}", response);
    Ok(service)
}

pub struct TcpTransport {
    address: SocketAddr,
    stream: TcpStream,
    next_msg: Vec<u8>,
}

impl TcpTransport {
    pub fn new(address: SocketAddr, stream: TcpStream) -> Self {
        // TODO rewrite on tokio.
        Self {
            address,
            stream,
            next_msg: Vec::new(),
        }
    }

    fn read_packet_kind(&mut self) -> nb::Result<PacketKind, anyhow::Error> {
        let mut msg_buf = [0_u8; PacketKind::PACKED_LEN];

        let bytes_read = match self.stream.read(&mut msg_buf) {
            Ok(bytes_read) if bytes_read > 0 => Ok(bytes_read),
            Err(err) => Err(nb::Error::Other(anyhow::Error::from(err))),
            _ => Err(nb::Error::WouldBlock),
        }?;
        self.next_msg.extend_from_slice(&msg_buf[..bytes_read]);

        if self.next_msg.len() >= PacketKind::PACKED_LEN {
            Ok(PacketKind::decode(&self.next_msg))
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl Transport for TcpTransport {
    type Error = anyhow::Error;
    type Address = SocketAddr;
    type Payload = Vec<u8>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<TransportEvent<Self::Address, Self::Payload>, Self::Error> {
        let kind = self.read_packet_kind()?;

        let packet = match kind {
            PacketKind::Payload(len) => {
                let mut payload = self.next_msg[PacketKind::PACKED_LEN..].to_vec();
                payload.resize(len, 0);

                self.stream
                    .read_exact(&mut payload)
                    .map_err(|e| nb::Error::Other(Self::Error::from(e)))?;

                TransportEvent::Packet {
                    address: self.address,
                    data: PacketData::Payload(payload),
                }
            }
            PacketKind::Confirmed => TransportEvent::Packet {
                address: self.address,
                data: PacketData::Confirmed,
            },
        };

        log::trace!("Received packet {:?}", packet);
        self.next_msg.clear();
        Ok(packet)
    }

    fn confirm_packet(&mut self, _from: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::Confirmed.to_bytes();

        log::trace!("Confirm packet: {:?}", packet);
        self.stream.write_all(packet.as_ref()).map_err(From::from)
    }

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        _to: Self::Address,
    ) -> Result<(), Self::Error> {
        let mut packet: Vec<u8> = Vec::new();
        packet.extend(PacketWithPayload::new(payload));

        log::trace!("Send packet: {:?}", packet);
        self.stream.write_all(packet.as_ref()).map_err(From::from)
    }
}
