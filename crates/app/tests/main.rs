#![feature(async_fn_in_trait)]

use std::time::Duration;

use cyberpixie_app::{
    core::proto::types::{DeviceInfo, DeviceRole, FirmwareInfo, Hertz, ImageId},
    App, Board, Configuration, CyberpixieError, CyberpixieResult,
};
use cyberpixie_embedded_storage::{
    test_utils::{leaked_buf, MemoryBackend},
    MemoryLayout, StorageImpl,
};
use cyberpixie_network::{
    tokio::{TokioConnection, TokioSocket, TokioStack},
    Client, NetworkStack,
};
use tokio::task::JoinHandle;

struct BoardStub {
    memory: Option<MemoryBackend>,
}

impl Default for BoardStub {
    fn default() -> Self {
        Self {
            memory: Some(MemoryBackend::default()),
        }
    }
}

impl Board for BoardStub {
    type Storage = StorageImpl<MemoryBackend>;
    type NetworkStack = TokioStack;
    type RenderTask = StorageImpl<MemoryBackend>;

    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
        let memory = self.memory.take()?;
        let layout = MemoryLayout {
            base: 0,
            size: memory.0.len() as u32,
        };
        Some((
            StorageImpl::init(Configuration::default(), memory, layout, leaked_buf(512)).unwrap(),
            TokioStack,
        ))
    }

    async fn start_rendering(
        &mut self,
        storage: Self::Storage,
        _image_id: ImageId,
    ) -> CyberpixieResult<Self::RenderTask> {
        Ok(storage)
    }

    async fn stop_rendering(
        &mut self,
        handle: Self::RenderTask,
    ) -> CyberpixieResult<Self::Storage> {
        Ok(handle)
    }

    fn firmware_info(&self) -> FirmwareInfo {
        FirmwareInfo
    }
}

async fn create_loopback(
    socket: &mut TokioSocket,
    port: u16,
) -> (JoinHandle<CyberpixieResult<()>>, Client<TokioConnection>) {
    let _ = env_logger::try_init();
    // Create a thread with an application instance
    let app = App::with_port(BoardStub::default(), port).unwrap();
    let app_handle = tokio::spawn(app.run());
    // Wait until the socket will be ready to listen a client connection.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create a Cyberpixie client and connect with an application.
    let client = Client::connect(
        socket,
        embedded_nal::SocketAddr::new(embedded_nal::Ipv6Addr::localhost().into(), port),
    )
    .await
    .expect("unable to establish client connection");
    (app_handle, client)
}

async fn device_info(client: &mut Client<TokioConnection>) -> DeviceInfo {
    client.peer_info().await.unwrap().device_info.unwrap()
}

#[tokio::test]
async fn test_simple_handshake() {
    let mut stack = TokioStack;
    let (_app, mut client) = create_loopback(&mut stack.socket(), 10_234).await;

    let info = client.peer_info().await.unwrap();
    assert_eq!(info.role, DeviceRole::Main);

    client.debug("Hello debug").await.unwrap();
    client.debug("Hello debug 2").await.unwrap();
    drop(client);
}

#[tokio::test]
async fn test_images_logic() {
    let image_data = [1_u8; 72];

    let mut stack = TokioStack;
    let (_app, mut client) = create_loopback(&mut stack.socket(), 10_235).await;

    // Add image and check the resulting device state.
    let id = client.add_image(Hertz(50), 24, &image_data).await.unwrap();
    assert_eq!(id, ImageId(0));
    assert_eq!(device_info(&mut client).await.images_count, ImageId(1));
    // Start a first image rendering.
    client.start(id).await.unwrap();
    let info = device_info(&mut client).await;
    assert!(info.active);
    assert_eq!(info.current_image, Some(id));

    // Try to send another image
    let id = client.add_image(Hertz(250), 24, &image_data).await.unwrap();
    let info = device_info(&mut client).await;
    // Make sure that device reports that there is no active rendering task.
    assert!(!info.active);
    assert_eq!(id, ImageId(1));

    // Start and stop second rendering.
    client.start(id).await.unwrap();
    client.stop().await.unwrap();
    let info = device_info(&mut client).await;
    assert!(!info.active);
    assert_eq!(info.current_image, Some(ImageId(1)));

    // Clear pictures stored in the device.
    client.clear_images().await.unwrap();
    let info = device_info(&mut client).await;
    assert_eq!(info.current_image, None);
    assert_eq!(info.images_count, ImageId(0));

    // Try to add in incorrect images
    assert_eq!(
        client.add_image(Hertz(250), 23, &image_data).await,
        Err(CyberpixieError::StripLengthMismatch),
    );
    assert_eq!(
        client.add_image(Hertz(250), 24, &image_data[0..7]).await,
        Err(CyberpixieError::ImageLengthMismatch),
    );

    // Try to show image
    assert_eq!(
        client.start(ImageId(0)).await,
        Err(CyberpixieError::ImageNotFound)
    );
}
