use std::{
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream},
};

use cyberpixie_proto::transport::{Packet, PacketData, PacketKind, Transport};

pub struct TransportImpl {
    address: SocketAddr,
    next_msg: Vec<u8>,
    stream: TcpStream,
}

impl TransportImpl {
    pub fn new(address: SocketAddr, stream: TcpStream) -> Self {
        Self {
            address,
            stream,
            next_msg: Vec::new(),
        }
    }

    fn read_some_bytes(&mut self) -> nb::Result<usize, anyhow::Error> {
        let bytes_read = match self.stream.read(&mut self.next_msg) {
            Ok(bytes_read) if bytes_read > 0 => Ok(bytes_read),

            Err(err) if err.kind() != ErrorKind::Interrupted => {
                Err(nb::Error::Other(anyhow::Error::from(err)))
            }
            _ => Err(nb::Error::WouldBlock),
        }?;

        Ok(bytes_read)
    }
}

impl Transport for TransportImpl {
    type Error = anyhow::Error;

    type Address = SocketAddr;

    type Payload = Vec<u8>;

    fn poll_next_packet(
        &mut self,
    ) -> nb::Result<Packet<Self::Address, Self::Payload>, Self::Error> {
        self.read_some_bytes()?;

        if self.next_msg.len() < PacketKind::PACKED_LEN {
            return Err(nb::Error::WouldBlock);
        }

        let kind = PacketKind::decode(&self.next_msg);
        match kind {
            PacketKind::Payload(len) if len == self.next_msg.len() - PacketKind::PACKED_LEN => {
                Ok(Packet {
                    address: self.address,
                    data: PacketData::Payload(self.next_msg.drain(..).collect::<Vec<_>>()),
                })
            }
            PacketKind::RequestNext => Ok(Packet {
                address: self.address,
                data: PacketData::RequestNext,
            }),

            _ => Err(nb::Error::WouldBlock),
        }
    }

    fn request_next_packet(&mut self, _from: Self::Address) -> Result<(), Self::Error> {
        let packet = PacketKind::RequestNext.to_bytes();
        self.stream.write_all(packet.as_ref()).map_err(From::from)
    }

    fn send_packet<P: AsRef<[u8]>>(
        &mut self,
        payload: P,
        _to: Self::Address,
    ) -> Result<(), Self::Error> {
        let mut packet: Vec<u8> = Vec::new();
        packet.extend_from_slice(
            PacketKind::Payload(payload.as_ref().len())
                .to_bytes()
                .as_ref(),
        );
        self.stream.write_all(packet.as_ref()).map_err(From::from)
    }
}
