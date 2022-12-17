use std::{
    convert::Infallible,
    net::{TcpListener, TcpStream},
    thread::JoinHandle,
};

use cyberpixie_core::{Config, DeviceService, DeviceStorage, Image};
use cyberpixie_proto::{
    types::{DeviceInfo, DeviceRole, ImageId},
    ExactSizeRead,
};
use cyberpixie_std_network::{Client, NetworkPart};
use embedded_io::{
    blocking::{Read, Seek},
    Io,
};

fn create_loopback<D>(device: D) -> anyhow::Result<(Client, JoinHandle<()>)>
where
    D: DeviceService + Send + 'static,
{
    let _ = env_logger::try_init();

    let listener = TcpListener::bind("0.0.0.0:0")?;
    let addr = listener.local_addr()?;

    let mut server = NetworkPart::new(device, listener)?;
    let handle = std::thread::spawn(move || loop {
        if let Err(nb::Error::Other(err)) = server.poll() {
            panic!("{err}");
        }
    });

    let client = Client::connect(TcpStream::connect(addr)?)?;
    Ok((client, handle))
}

struct DeviceStub;
struct StorageStub;
struct ImageReadStub;

impl DeviceService for DeviceStub {
    type Storage = StorageStub;

    fn device_info(&self) -> DeviceInfo {
        DeviceInfo {
            role: DeviceRole::Main,
            group_id: None,
            strip_len: Some(36),
        }
    }

    fn storage(&self) -> Self::Storage {
        todo!()
    }
}

impl Io for ImageReadStub {
    type Error = Infallible;
}

impl Read for ImageReadStub {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, Self::Error> {
        unimplemented!()
    }
}

impl ExactSizeRead for ImageReadStub {
    fn bytes_remaining(&self) -> usize {
        0
    }
}

impl Seek for ImageReadStub {
    fn seek(&mut self, _pos: embedded_io::SeekFrom) -> Result<u64, Self::Error> {
        unimplemented!()
    }
}

impl DeviceStorage for StorageStub {
    type Error = Infallible;

    type ImageRead<'a> = ImageReadStub;

    fn config(&self) -> Result<Config, Self::Error> {
        unimplemented!()
    }

    fn set_config(&self, _value: &Config) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn add_image<R>(
        &self,
        _refresh_rate: cyberpixie_proto::types::Hertz,
        _image: R,
    ) -> Result<ImageId, cyberpixie_core::AddImageError<R::Error, Self::Error>>
    where
        R: ExactSizeRead,
    {
        todo!()
    }

    fn read_image(&self, _id: ImageId) -> Result<Option<Image<Self::ImageRead<'_>>>, Self::Error> {
        unimplemented!()
    }

    fn images_count(&self) -> Result<u16, Self::Error> {
        unimplemented!()
    }

    fn clear_images(&self) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

#[test]
fn test_simple_handshake() {
    let (mut client, _device) = create_loopback(DeviceStub).unwrap();
    assert_eq!(
        client.device_info,
        DeviceInfo {
            role: DeviceRole::Main,
            group_id: None,
            strip_len: Some(36),
        }
    );

    let info = client.handshake().unwrap();
    assert_eq!(info, client.device_info);

    client.debug("Hello debug").unwrap();
    client.debug("Hello debug 2").unwrap();
}
