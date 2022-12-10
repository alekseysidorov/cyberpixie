use embedded_io::blocking::Read;

pub use messages::MessageHeader;
pub use types::{DeviceRole, FirmwareInfo, Handshake, Hertz, ImageId, ImageInfo};

mod messages;
pub mod transport;
mod types;

#[derive(Debug, Clone, Copy)]
pub struct PayloadReader<T: Read> {
    pub len: usize,
    pub inner: T,
}

pub trait Peer {
    type Address;
}

pub trait Service: Peer {
    fn handle_connect(
        &mut self,
        peer: Self::Address,
        handshake: Handshake,
    ) ->  Result<Handshake, anyhow::Error>;

    fn handle_disconnect(&mut self, peer: Self::Address) -> Result<(), anyhow::Error>;

    fn handle_add_image<T: Read>(
        &mut self,
        peer: Self::Address,
        info: ImageInfo,
        body: PayloadReader<T>,
    ) -> Result<ImageId, anyhow::Error>;
}

pub trait Client: Peer {
    fn connect(&mut self, peer: Self::Address, handshake: Handshake) -> Result<Handshake, anyhow::Error>;

    fn add_image<T: Read>(&mut self, peer: Self::Address, info: ImageInfo, body: PayloadReader<T>) -> Result<ImageId, anyhow::Error>;

    fn disconnect(&mut self, peer: Self::Address) -> Result<(), anyhow::Error>;
}
