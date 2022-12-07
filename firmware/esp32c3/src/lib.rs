use embedded_svc::storage::RawStorage;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys::EspError;
use serde::{de::DeserializeOwned, Serialize};

pub struct PostCard;

pub mod wifi;

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

pub struct DefaultStorage {
    pub inner: EspNvs<NvsDefault>,
}

impl DefaultStorage {
    const NAMESPACE: &'static str = "images";

    pub fn take() -> Result<Self, EspError> {
        let partition = EspNvsPartition::<NvsDefault>::take()?;
        let nvs = EspNvs::new(partition, Self::NAMESPACE, true)?;

        Ok(Self { inner: nvs })
    }

    pub fn set<T: Serialize>(&mut self, name: &str, value: &T) -> anyhow::Result<()> {
        let buf = postcard::to_stdvec(value)?;
        self.inner.set_raw(&format!("v.{name}"), &buf)?;

        let mut size_buf = [0_u8; 8];
        self.inner.set_raw(
            &format!("s.{name}"),
            postcard::to_slice(&buf.len(), &mut size_buf)?,
        )?;
        Ok(())
    }

    pub fn get<T>(&self, name: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut size_buf = [0_u8; 8];
        let size_buf = self.inner.get_raw(&format!("s.{name}"), &mut size_buf)?;
        let size = if let Some(size_buf) = size_buf {
            postcard::from_bytes(size_buf)?
        } else {
            return Ok(None);
        };

        let mut buf = vec![0_u8; size];
        let buf = self.inner.get_raw(&format!("v.{name}"), &mut buf)?;
        if let Some(buf) = buf {
            let value = postcard::from_bytes(buf)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn remove(&mut self, name: &str) -> anyhow::Result<()> {
        self.inner.remove(&format!("s.{name}"))?;
        self.inner.remove(&format!("v.{name}"))?;
        Ok(())
    }
}
