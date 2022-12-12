use cyberpixie_proto::types::Hertz;
use embedded_svc::storage::StorageImpl;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys::EspError;
use serde::{Deserialize, Serialize};

struct PostCard;

const BLOCK_SIZE: usize = 512;

impl embedded_svc::storage::SerDe for PostCard {
    type Error = postcard::Error;

    fn serialize<'a, T>(&self, buf: &'a mut [u8], value: &T) -> Result<&'a [u8], Self::Error>
    where
        T: serde::Serialize,
    {
        let out = postcard::to_slice(value, buf)?;
        Ok(out)
    }

    fn deserialize<T>(&self, buf: &[u8]) -> Result<T, Self::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        postcard::from_bytes(buf)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StorageMeta {
    images_count: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct ImageHeader {
    image_len: u32,
    refresh_rate: Hertz,
}

pub struct ImagesRegistry {
    storage: StorageImpl<BLOCK_SIZE, EspNvs<NvsDefault>, PostCard>,
}

impl ImagesRegistry {
    const NAMESPACE: &'static str = "images";

    pub fn take() -> Result<Self, EspError> {
        let partition = EspNvsPartition::<NvsDefault>::take()?;
        let nvs = EspNvs::new(partition, Self::NAMESPACE, true)?;

        Ok(Self {
            storage: StorageImpl::new(nvs, PostCard),
        })
    }
}
