#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self, clippy::missing_const_for_fn)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use cyberpixie_app::{Board, Storage, Configuration};
use cyberpixie_core::proto::types::FirmwareInfo;
use smart_leds::{SmartLedsWrite, RGB8};
use storage::ImagesRegistry;

pub mod render;
pub mod splash;
pub mod storage;
pub mod wifi;

/// Default device strip length.
pub const STRIP_LEN: usize = 24;
/// Default device configuration.
pub const DEFAULT_DEVICE_CONFIG: Configuration = Configuration {
    strip_len: STRIP_LEN as u16,
    current_image: None,
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
    pub const fn new(storage: ImagesRegistry, render: R) -> anyhow::Result<Self> {
        Ok(Self {
            storage,
            render: Some(render),
        })
    }
}

impl<R> Board for DeviceImpl<R>
where
    R: SmartLedsWrite<Color = RGB8> + Send + 'static,
    R::Error: std::fmt::Debug + std::error::Error + Send + Sync + 'static,
{
    type Storage = ImagesRegistry;
    type NetworkStack = std_embedded_nal::Stack;
    type RenderTask = render::Handle<R, ImagesRegistry>;

    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)> {
        Some((self.storage, std_embedded_nal::Stack::default()))
    }

    fn start_rendering(
        &mut self,
        storage: Self::Storage,
        image_id: cyberpixie_core::proto::types::ImageId,
    ) -> cyberpixie_core::Result<Self::RenderTask> {
        let Some(render) = self.render.take() else {
            return Err(cyberpixie_core::Error::ImageRenderIsBusy)
        };

        let refresh_rate = storage.read_image(image_id)?.refresh_rate;
        let handle = render::start_rendering(render, self.storage, image_id, refresh_rate)
            .map_err(cyberpixie_core::Error::internal)?;
        Ok(handle)
    }

    fn stop_rendering(
        &mut self,
        handle: Self::RenderTask,
    ) -> cyberpixie_core::Result<Self::Storage> {
        let (render, storage) = handle.stop().map_err(cyberpixie_core::Error::internal)?;
        self.render = Some(render);
        Ok(storage)
    }

    fn firmware_info(&self) -> FirmwareInfo {
        FirmwareInfo
    }
}
