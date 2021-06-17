use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use cyberpixie_proto::{
    DeviceRole, Handshake, PacketData, PacketKind, PacketWithPayload, Service, Transport,
    TransportEvent,
};

use crate::display_err;

const TIMEOUT: Duration = Duration::from_secs(60);
const HOST_DEVICE_ID: [u32; 4] = [0; 4];

pub fn connect_to(addr: &SocketAddr) -> anyhow::Result<UdpSocket> {
    log::debug!("Connecting to the {}", addr);
    let socket = UdpSocket::bind("0.0.0.0:34254")?;
    socket.set_read_timeout(Some(TIMEOUT))?;
    socket.set_write_timeout(Some(TIMEOUT))?;
    socket.connect(addr)?;
    log::debug!("Connected with the {}", addr);
    Ok(socket)
}

pub fn create_service(addr: SocketAddr) -> anyhow::Result<Service<UdpTransport>> {
    let stream = connect_to(&addr)?;
    let transport = UdpTransport::new(addr, stream);

    let mut service = Service::new(transport, 512);
    // let response = service
    //     .handshake(
    //         addr,
    //         Handshake {
    //             device_id: HOST_DEVICE_ID,
    //             group_id: None,
    //             role: DeviceRole::Host,
    //         },
    //     )?
    //     .map_err(display_err)?;
    // log::trace!("Connected with device: {:?}", response);
    Ok(service)
}

pub struct UdpTransport {
    address: SocketAddr,
    socket: UdpSocket,
}

impl UdpTransport {
    pub fn new(address: SocketAddr, socket: UdpSocket) -> Self {
        Self { address, socket }
    }

    fn read_packet(&mut self) -> Result<Vec<u8>, anyhow::Error> {
        let mut msg_buf = vec![0; 1024];
        let bytes_read = match self.socket.recv(&mut msg_buf) {
            Ok(received) => Ok(received),
            Err(e) => Err(e),
        }?;
        msg_buf.truncate(bytes_read);
        Ok(msg_buf)
    }
}

impl Transport for UdpTransport {
    type Error = anyhow::Error;
    type Address = SocketAddr;
    type Payload = Vec<u8>;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<TransportEvent<Self::Address, Self::Payload>, Self::Error> {
        let msg_bytes = self.read_packet()?;
        let packet = match PacketKind::decode(&msg_bytes) {
            PacketKind::Payload(len) => {
                let payload = msg_bytes[PacketKind::PACKED_LEN..].to_vec();
                assert_eq!(payload.len(), len);
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
        Ok(packet)
    }

    fn confirm_packet(&mut self, _from: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::Confirmed.to_bytes();

        std::thread::sleep_ms(5000);

        log::trace!("Confirm packet: {:?}", packet);
        self.socket
            .send(packet.as_ref())
            .map_err(From::from)
            .map(drop)
    }

    fn send_packet<P: Iterator<Item = u8> + ExactSizeIterator>(
        &mut self,
        payload: P,
        _to: Self::Address,
    ) -> Result<(), Self::Error> {
        let mut packet: Vec<u8> = Vec::new();
        packet.extend(PacketWithPayload::new(payload));

        log::trace!("Send packet: {:?}", packet);
        self.socket
            .send(packet.as_ref())
            .map_err(From::from)
            .map(drop)
    }
}
