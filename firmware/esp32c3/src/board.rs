//! Board support

use cyberpixie_app::{
    asynch::Board,
    core::proto::types::{FirmwareInfo, ImageId},
    Configuration,
};
use embassy_net::Stack;
use esp_storage::FlashStorage;
use esp_wifi::wifi::WifiDevice;

use crate::{
    network::NetworkStackImpl, render::RenderingHandle, singleton, StorageImpl,
    DEFAULT_MEMORY_LAYOUT,
};

/// Board support implementation for the Cyberpixie device.
pub struct BoardImpl {
    network: Option<NetworkStackImpl>,
    storage: Option<StorageImpl>,
    rendering_handle: RenderingHandle,
}

impl BoardImpl {
    pub fn new(
        stack: &'static Stack<WifiDevice<'static>>,
        rendering_handle: RenderingHandle,
    ) -> Self {
        let storage = StorageImpl::init(
            Configuration::default(),
            FlashStorage::new(),
            DEFAULT_MEMORY_LAYOUT,
            singleton!([0_u8; 512]),
        )
        .expect("Unable to create storage");

        Self {
            network: Some(NetworkStackImpl::new(stack)),
            storage: Some(storage),
            rendering_handle,
        }
    }
}

impl Board for BoardImpl {
    type Storage = StorageImpl;
    type NetworkStack = NetworkStackImpl;
    type RenderTask = RenderingHandle;

    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
        let storage = self.storage.take()?;
        let stack = self.network.take()?;
        Some((storage, stack))
    }

    async fn start_rendering(
        &mut self,
        storage: Self::Storage,
        image_id: ImageId,
    ) -> cyberpixie_app::CyberpixieResult<Self::RenderTask> {
        self.rendering_handle.start(storage, image_id).await;
        Ok(self.rendering_handle.clone())
    }

    async fn stop_rendering(
        &mut self,
        handle: Self::RenderTask,
    ) -> cyberpixie_app::CyberpixieResult<Self::Storage> {
        Ok(handle.stop().await)
    }

    fn firmware_info(&self) -> FirmwareInfo {
        FirmwareInfo
    }
}
