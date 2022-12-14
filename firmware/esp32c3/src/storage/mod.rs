use std::sync::Mutex;

use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use cyberpixie_storage::{Config, DeviceStorage, Image};
use embedded_svc::storage::RawStorage;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys::EspError;
use log::info;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use self::block_reader::BlockReader;

mod block_reader;

struct PostCard;

/// Read/write block size.
const BLOCK_SIZE: usize = 512;
/// Image registry namespace
const STORAGE_NAMESPACE: &str = "images";

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
struct ImageHeader {
    image_len: u32,
    refresh_rate: Hertz,
}

static STORAGE: Lazy<Mutex<EspNvs<NvsDefault>>> = Lazy::new(|| {
    let partition = EspNvsPartition::<NvsDefault>::take().unwrap();
    let esp = EspNvs::new(partition, STORAGE_NAMESPACE, true).unwrap();
    Mutex::new(esp)
});

#[derive(Debug, Clone, Copy, Default)]
pub struct ImagesRegistry;

impl ImagesRegistry {
    pub fn new() -> Self {
        Self
    }

    fn set<T>(&self, name: &str, value: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let mut buf = [0_u8; BLOCK_SIZE];

        postcard::to_slice(value, &mut buf)?;
        self.set_raw(name, &buf)?;
        Ok(())
    }

    fn get<T>(&self, name: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut buf = [0_u8; BLOCK_SIZE];

        let bytes = self.get_raw(name, &mut buf)?;
        bytes
            .map(|buf| postcard::from_bytes(buf))
            .transpose()
            .map_err(From::from)
    }

    fn set_raw(&self, name: &str, buf: &[u8]) -> Result<bool, EspError> {
        STORAGE.lock().unwrap().set_raw(name, buf)
    }

    fn get_raw<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, EspError> {
        STORAGE.lock().unwrap().get_raw(name, buf)
    }

    fn set_images_count(&self, count: u16) -> Result<(), anyhow::Error> {
        self.set("img.count", &count)
    }
}

// impl embedded_io::Io for ImagesRegistry {
//     type Error = std::io::Error;
// }

impl DeviceStorage for ImagesRegistry {
    type ImageRead<'a> = BlockReader<'a>;
    type Error = anyhow::Error;

    fn config(&self) -> Result<Config, Self::Error> {
        self.get("config").map(Option::unwrap_or_default)
    }

    fn set_config(&self, value: &Config) -> Result<(), Self::Error> {
        self.set("config", value)
    }

    fn images_count(&self) -> Result<u16, Self::Error> {
        self.get("img.count").map(Option::unwrap_or_default)
    }

    fn add_image<R>(&self, refresh_rate: Hertz, mut image: R) -> Result<ImageId, Self::Error>
    where
        Self::Error: From<R::Error>,
        R: ExactSizeRead,
    {
        let idx = self.images_count()?;

        // Save image header.
        let header = ImageHeader {
            image_len: image.bytes_remaining() as u32,
            refresh_rate,
        };
        self.set(&format!("img.{idx}.header"), &header)?;
        info!("Saving image with header: {header:?}");

        // Save image content.
        let mut buf = [0_u8; BLOCK_SIZE];

        let blocks = image.bytes_remaining() / BLOCK_SIZE;
        for block in 0..=blocks {
            let to = std::cmp::min(image.bytes_remaining(), BLOCK_SIZE);
            image
                .read_exact(&mut buf[0..to])
                .map_err(|err| anyhow::anyhow!("Unable to read image: {err}"))?;
            self.set_raw(&format!("img.{idx}.block.{block}"), &buf[0..to])?;
            info!("Write block {block} -> [0..{to}]");
        }

        let id = ImageId(idx);
        // self.set_images_count(idx + 1)?;
        Ok(id)
    }

    fn read_image(&self, idx: ImageId) -> Result<Option<Image<Self::ImageRead<'_>>>, Self::Error> {
        let images_count = self.images_count()?;
        if idx.0 >= images_count {
            return Ok(None);
        }

        // Read image header.
        let header: ImageHeader = self.get("img.{idx}.header")?.expect("storage corrupted");
        // Create image block reader.
        let image = Image {
            refresh_rate: header.refresh_rate,
            bytes: BlockReader::new(self, idx, header.image_len)?,
        };
        Ok(Some(image))
    }
}
