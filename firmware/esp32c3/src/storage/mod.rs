use std::sync::Mutex;

use cyberpixie_core::storage::DEFAULT_BLOCK_SIZE;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use once_cell::sync::Lazy;

pub use self::registry::ImagesRegistry;

mod registry;

const BLOCK_SIZE: usize = DEFAULT_BLOCK_SIZE;

/// Pictures registry namespace
const STORAGE_NAMESPACE: &str = "storage";

static STORAGE: Lazy<Mutex<EspNvs<NvsDefault>>> = Lazy::new(|| {
    let partition = EspNvsPartition::<NvsDefault>::take().unwrap();
    let esp = EspNvs::new(partition, STORAGE_NAMESPACE, true).unwrap();
    Mutex::new(esp)
});

struct PostCard;

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
