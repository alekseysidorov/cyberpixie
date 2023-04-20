pub use cyberpixie_core::service::DeviceConfig;

pub mod render;
pub mod splash;
pub mod storage;
pub mod wifi;

/// Default device configuration.
pub const DEFAULT_DEVICE_CONFIG: DeviceConfig = DeviceConfig { strip_len: 24 };

#[derive(Debug)]
enum ImageRenderingState<R, D> {
    Idle { render: R },
    Running { handle: render::Handle<R, D> }
}

pub struct DeviceImpl {
}
