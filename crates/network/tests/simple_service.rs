use std::{
    convert::Infallible,
    net::{TcpListener, TcpStream},
    thread::JoinHandle,
};

use cyberpixie_core::{
    proto::types::{DeviceInfo, DeviceRole, Hertz, ImageId, PeerInfo},
    service::{DeviceConfig, DeviceImage, DeviceService, DeviceStorage},
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
    type ImageRender = ();

    fn peer_info(&self) -> cyberpixie_core::Result<PeerInfo> {
        Ok(PeerInfo {
            role: DeviceRole::Main,
            group_id: None,
            device_info: Some(DeviceInfo::empty(36)),
        })
    }

    fn storage(&self) -> Self::Storage {
        unimplemented!()
    }

    fn show_current_image(&mut self) -> cyberpixie_core::Result<Self::ImageRender> {
        unimplemented!()
    }

    fn hide_current_image(&mut self, _task: Self::ImageRender) -> cyberpixie_core::Result<()> {
        unimplemented!()
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
    type ImageRead<'a> = ImageReadStub;

    fn config(&self) -> cyberpixie_core::Result<DeviceConfig> {
        unimplemented!()
    }

    fn set_config(&self, _value: &DeviceConfig) -> cyberpixie_core::Result<()> {
        unimplemented!()
    }

    fn add_image<R: ExactSizeRead>(
        &self,
        _refresh_rate: Hertz,
        _image: R,
    ) -> cyberpixie_core::Result<ImageId> {
        unimplemented!()
    }

    fn read_image(&self, _id: ImageId) -> cyberpixie_core::Result<DeviceImage<'_, Self>> {
        unimplemented!()
    }

    fn images_count(&self) -> cyberpixie_core::Result<ImageId> {
        unimplemented!()
    }

    fn clear_images(&self) -> cyberpixie_core::Result<()> {
        unimplemented!()
    }

    fn set_current_image_id(&self, _id: ImageId) -> cyberpixie_core::Result<()> {
        unimplemented!()
    }

    fn current_image_id(&self) -> cyberpixie_core::Result<Option<ImageId>> {
        unimplemented!()
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_simple_handshake() {
    let (mut client, _device) = create_loopback(DeviceStub).unwrap();
    assert_eq!(
        client.peer_info,
        PeerInfo {
            role: DeviceRole::Main,
            group_id: None,
            device_info: Some(DeviceInfo::empty(36)),
        }
    );

    let info = client.handshake().unwrap();
    assert_eq!(info, client.peer_info);

    client.debug("Hello debug").unwrap();
    client.debug("Hello debug 2").unwrap();
}
