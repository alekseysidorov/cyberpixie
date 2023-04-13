use cyberpixie_core::service::Config;

pub mod render;
pub mod splash;
pub mod storage;
pub mod wifi;

/// Default device configuration.
pub const DEFAULT_CONFIG: Config = Config { strip_len: 48 };
