use std::sync::Mutex;

use cyberpixie_core::{image_reader::BLOCK_SIZE, Config, DeviceStorage, Image};
use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use embedded_svc::storage::RawStorage;
use esp_idf_svc::nvs::{EspNvs, EspNvsPartition, NvsDefault};
use esp_idf_sys::EspError;
use log::info;
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use self::image_reader::{BlockReaderImpl, ImageReader};

mod image_reader;

struct PostCard;

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

    fn remove(&self, name: &str) -> Result<bool, EspError> {
        info!("Removing '{name}' entry...");
        STORAGE.lock().unwrap().remove(name)
    }

    fn set_images_count(&self, count: u16) -> Result<(), anyhow::Error> {
        self.set("img.count", &count)
    }

    fn read_image_header(&self, image_index: ImageId) -> Result<ImageHeader, anyhow::Error> {
        self.get(&format!("img.{image_index}.header"))?
            .ok_or_else(|| anyhow::anyhow!("Unable to read image header: storage corrupted"))
    }
}

impl DeviceStorage for ImagesRegistry {
    type ImageRead<'a> = ImageReader<'a>;
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
        let image_index = self.images_count()?;

        // Save image header.
        let header = ImageHeader {
            image_len: image.bytes_remaining() as u32,
            refresh_rate,
        };
        self.set(&format!("img.{image_index}.header"), &header)?;
        info!("Saving image with header: {header:?}");

        // Save image content.
        let mut buf = [0_u8; BLOCK_SIZE];

        let blocks = image.bytes_remaining() / BLOCK_SIZE;
        for block in 0..=blocks {
            let to = std::cmp::min(image.bytes_remaining(), BLOCK_SIZE);
            image
                .read_exact(&mut buf[0..to])
                .map_err(|err| anyhow::anyhow!("Unable to read image: {err}"))?;
            self.set_raw(&format!("img.{image_index}.block.{block}"), &buf[0..to])?;
            info!("Write block {block} -> [0..{to}]");
        }

        let id = ImageId(image_index);
        self.set_images_count(image_index + 1)?;
        info!("Image saved, total images count: {}", image_index + 1);
        Ok(id)
    }

    fn read_image(
        &self,
        image_index: ImageId,
    ) -> Result<Option<Image<Self::ImageRead<'_>>>, Self::Error> {
        let images_count = self.images_count()?;

        if image_index.0 >= images_count {
            return Ok(None);
        }

        let header = self.read_image_header(image_index)?;
        let image = Image {
            refresh_rate: header.refresh_rate,
            bytes: ImageReader::new(
                BlockReaderImpl::new(self, image_index),
                header.image_len as usize,
            ),
        };
        Ok(Some(image))
    }

    fn clear_images(&self) -> Result<(), Self::Error> {
        let images_count = self.images_count()?;

        info!("Deleting {images_count} images...");
        for image_index in 0..images_count {
            let header = self.read_image_header(ImageId(image_index))?;
            // Remove image blocks.
            let blocks_count = header.image_len as usize / BLOCK_SIZE;
            for block in 0..=blocks_count {
                self.remove(&format!("img.{image_index}.block.{block}"))?;
            }
            // Remove image header.
            self.remove(&format!("img.{image_index}.header"))?;
        }
        // Reset images counter.
        self.set_images_count(0)?;

        Ok(())
    }
}
