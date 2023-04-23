pub use cyberpixie_core::service::DeviceConfig;
use cyberpixie_core::{
    proto::types::{DeviceInfo, DeviceRole, PeerInfo},
    service::{DeviceService, DeviceStorage},
};
use smart_leds::{SmartLedsWrite, RGB8};
use storage::ImagesRegistry;

pub mod render;
pub mod splash;
pub mod storage;
pub mod wifi;

/// Default device strip length.
pub const STRIP_LEN: usize = 24;
/// Default device configuration.
pub const DEFAULT_DEVICE_CONFIG: DeviceConfig = DeviceConfig {
    strip_len: STRIP_LEN as u16,
};
/// LED pin used by the device.
pub const LED_PIN: u32 = 8;

#[derive(Debug)]
pub struct DeviceImpl<R> {
    storage: ImagesRegistry,
    render: Option<R>,
}

impl<R> DeviceImpl<R>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    R::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
{
    pub fn new(storage: ImagesRegistry, render: R) -> anyhow::Result<Self> {
        Ok(Self {
            storage,
            render: Some(render),
        })
    }
}

impl<R> DeviceService for DeviceImpl<R>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    R::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
{
    type Storage = ImagesRegistry;
    type ImageRender = render::Handle<R, ImagesRegistry>;

    fn peer_info(&self) -> cyberpixie_core::Result<PeerInfo> {
        let device_info = DeviceInfo {
            strip_len: self.storage.config()?.strip_len,
            images_count: self.storage.images_count()?,
            current_image: self.storage.current_image_id()?,
            active: self.render.is_none(),
        };

        Ok(PeerInfo {
            role: DeviceRole::Main,
            group_id: None,
            device_info: Some(device_info),
        })
    }

    fn storage(&self) -> Self::Storage {
        self.storage
    }

    fn show_current_image(&mut self) -> cyberpixie_core::Result<Self::ImageRender> {
        let Some(render) = self.render.take() else {
            return Err(cyberpixie_core::Error::ImageRenderIsBusy)
        };

        let refresh_rate = self.storage.read_current_image()?.refresh_rate;
        let handle = render::start_rendering(render, self.storage, refresh_rate)
            .map_err(cyberpixie_core::Error::internal)?;
        Ok(handle)
    }

    fn hide_current_image(&mut self, task: Self::ImageRender) -> cyberpixie_core::Result<()> {
        let (render, _storage) = task.stop().map_err(cyberpixie_core::Error::internal)?;
        self.render = Some(render);
        Ok(())
    }
}
