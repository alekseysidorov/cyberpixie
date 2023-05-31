use core::{fmt::Display, str::FromStr, time::Duration};

use endian_codec::{DecodeLE, EncodeLE, PackedSize};
use postcard::experimental::max_size::MaxSize;
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Serialize, Deserialize, MaxSize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum DeviceRole {
    /// A control device such as a telephone or laptop.
    Client = 0,
    /// A main device that receives commands directly from the client and then re-sends them
    /// to the slave devices if they exist.
    Main = 1,
    /// A secondary device that executes commands from the main one.
    Secondary = 2,
}

impl Display for DeviceRole {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Client => f.write_str("client"),
            Self::Main => f.write_str("main"),
            Self::Secondary => f.write_str("secondary"),
        }
    }
}

#[derive(Serialize, Deserialize, MaxSize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct PeerInfo {
    pub role: DeviceRole,
    pub group_id: Option<u32>,
    pub device_info: Option<DeviceInfo>,
}

impl PeerInfo {
    #[must_use]
    pub const fn client() -> Self {
        Self {
            role: DeviceRole::Client,
            group_id: None,
            device_info: None,
        }
    }
}

#[derive(Serialize, Deserialize, MaxSize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct DeviceInfo {
    pub strip_len: u16,
    pub images_count: ImageId,
    pub current_image: Option<ImageId>,
    /// Indicates whether there is an active image rendering task.
    pub active: bool,
}

impl DeviceInfo {
    #[must_use]
    pub const fn empty(strip_len: u16) -> Self {
        Self {
            strip_len,
            images_count: ImageId(0),
            current_image: None,
            active: false,
        }
    }
}

#[derive(Serialize, Deserialize, MaxSize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct ImageInfo {
    pub refresh_rate: Hertz,
    pub strip_len: u16,
}

#[derive(Serialize, Deserialize, MaxSize, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FirmwareInfo;

#[derive(
    Serialize,
    Deserialize,
    MaxSize,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    PartialOrd,
    Ord,
    Hash,
    Default,
    PackedSize,
)]
pub struct Hertz(pub u32);

#[derive(
    Serialize,
    Deserialize,
    MaxSize,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    PartialOrd,
    Ord,
    Hash,
    Default,
)]
pub struct ImageId(pub u16);

impl FromStr for Hertz {
    type Err = <u32 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str(s).map(Self)
    }
}

impl From<u32> for Hertz {
    fn from(inner: u32) -> Self {
        Self(inner)
    }
}

impl EncodeLE for Hertz {
    fn encode_as_le_bytes(&self, bytes: &mut [u8]) {
        self.0.encode_as_le_bytes(bytes);
    }
}

impl DecodeLE for Hertz {
    fn decode_from_le_bytes(bytes: &[u8]) -> Self {
        Self(u32::decode_from_le_bytes(bytes))
    }
}

impl From<Hertz> for Duration {
    fn from(value: Hertz) -> Self {
        Self::from_secs_f64(1.0_f64 / f64::from(value.0))
    }
}

impl Display for ImageId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for Hertz {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}
