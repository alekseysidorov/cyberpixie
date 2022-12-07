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

pub trait Service {
    type Error;
    type Address;

    fn handle_connect(
        &mut self,
        peer: Self::Address,
        handshake: Handshake,
    ) -> Result<Handshake, Self::Error>;

    fn handle_disconnect(&mut self, peer: Self::Address) -> Result<(), Self::Error>;

    fn handle_add_image<T: Read>(
        &mut self,
        peer: Self::Address,
        info: ImageInfo,
        payload: PayloadReader<T>,
    ) -> Result<ImageId, Self::Error>;
}
