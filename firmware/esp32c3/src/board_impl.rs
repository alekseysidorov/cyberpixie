//! Board support implementation

use cyberpixie_app::{
    asynch::Board, core::proto::types::FirmwareInfo, Configuration, CyberpixieError,
};
use cyberpixie_embedded_storage::StorageImpl;
use embassy_net::Stack;
use esp_storage::FlashStorage;
use esp_wifi::wifi::WifiDevice;

use crate::{network::NetworkStackImpl, singleton, DEFAULT_MEMORY_LAYOUT};

/// Board support implementation for the Cyberpixie device.
pub struct BoardImpl {
    network: Option<NetworkStackImpl>,
    storage: Option<StorageImpl<FlashStorage>>,
}

impl BoardImpl {
    pub fn new(stack: &'static Stack<WifiDevice<'static>>) -> Self {
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
        }
    }
}

impl Board for BoardImpl {
    type Storage = StorageImpl<FlashStorage>;
    type NetworkStack = NetworkStackImpl;
    type RenderTask = ();

    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
        let storage = self.storage.take()?;
        let stack = self.network.take()?;
        Some((storage, stack))
    }

    fn start_rendering(
        &mut self,
        storage: Self::Storage,
        image_id: cyberpixie_app::core::proto::types::ImageId,
    ) -> cyberpixie_app::CyberpixieResult<Self::RenderTask> {
        self.storage = Some(storage);
        Ok(())
    }

    fn stop_rendering(
        &mut self,
        handle: Self::RenderTask,
    ) -> cyberpixie_app::CyberpixieResult<Self::Storage> {
        self.storage.take().ok_or(CyberpixieError::Internal)
    }

    fn firmware_info(&self) -> FirmwareInfo {
        FirmwareInfo
    }
}
