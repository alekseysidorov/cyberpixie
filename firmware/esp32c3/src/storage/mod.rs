use std::sync::Mutex;

use cyberpixie_core::{
    proto::types::{Hertz, ImageId},
    service::{Config, DeviceImage, DeviceStorage, Image},
    storage::DEFAULT_BLOCK_SIZE,
    Error as CyberpixieError, ExactSizeRead,
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

const BLOCK_SIZE: usize = DEFAULT_BLOCK_SIZE;

/// Image registry namespace
const STORAGE_NAMESPACE: &str = "images";

const DEFAULT_CONFIG: Config = Config { strip_len: 48 };

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

    fn set<T>(&self, name: &str, value: &T) -> cyberpixie_core::Result<()>
    where
        T: Serialize,
    {
        let mut buf = [0_u8; BLOCK_SIZE];

        postcard::to_slice(value, &mut buf).map_err(CyberpixieError::encode)?;
        self.set_raw(name, &buf)
            .map_err(CyberpixieError::storage_write)?;
        Ok(())
    }

    fn get<T>(&self, name: &str) -> Result<Option<T>, CyberpixieError>
    where
        T: DeserializeOwned,
    {
        let mut buf = [0_u8; BLOCK_SIZE];

        let bytes = self
            .get_raw(name, &mut buf)
            .map_err(CyberpixieError::storage_read)?;

        bytes
            .map(|buf| postcard::from_bytes(buf))
            .transpose()
            .map_err(CyberpixieError::decode)
    }

    fn set_raw(&self, name: &str, buf: &[u8]) -> cyberpixie_core::Result<bool> {
        STORAGE
            .lock()
            .unwrap()
            .set_raw(name, buf)
            .map_err(CyberpixieError::storage_write)
    }

    fn get_raw<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, EspError> {
        STORAGE.lock().unwrap().get_raw(name, buf)
    }

    fn remove(&self, name: &str) -> cyberpixie_core::Result<bool> {
        info!("Removing '{name}' entry...");
        STORAGE
            .lock()
            .unwrap()
            .remove(name)
            .map_err(CyberpixieError::storage_write)
    }

    fn set_images_count(&self, count: u16) -> Result<(), CyberpixieError> {
        self.set("img.count", &count)
            .map_err(CyberpixieError::storage_write)
    }

    fn read_image_header(&self, image_index: ImageId) -> cyberpixie_core::Result<ImageHeader> {
        self.get(&format!("img.{image_index}.header"))?
            .ok_or(CyberpixieError::StorageRead)
    }
}

impl DeviceStorage for ImagesRegistry {
    type ImageRead<'a> = ImageReader<'a>;

    fn config(&self) -> cyberpixie_core::Result<Config> {
        let config = self.get("config")?;
        Ok(config.unwrap_or(DEFAULT_CONFIG))
    }

    fn set_config(&self, value: &Config) -> cyberpixie_core::Result<()> {
        self.set("config", value)
            .map_err(CyberpixieError::storage_write)
    }

    fn images_count(&self) -> cyberpixie_core::Result<ImageId> {
        self.get("img.count")
            .map(Option::unwrap_or_default)
            .map_err(CyberpixieError::storage_read)
    }

    fn set_current_image(&self, id: ImageId) -> cyberpixie_core::Result<()> {
        self.set("img.current", &id)
            .map_err(CyberpixieError::storage_write)
    }

    fn current_image(&self) -> cyberpixie_core::Result<Option<ImageId>> {
        // There is no images in this registry, so the current image doesn't make sense.
        if self.images_count()?.0 == 0 {
            return Ok(None);
        }

        self.get("img.current")
            .map(Option::unwrap_or_default)
            .map_err(CyberpixieError::storage_read)
    }

    fn add_image<R>(&self, refresh_rate: Hertz, mut image: R) -> cyberpixie_core::Result<ImageId>
    where
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
                .map_err(CyberpixieError::storage_write)?;

            self.set_raw(&format!("img.{image_index}.block.{block}"), &buf[0..to])?;
            info!("Write block {block} -> [0..{to}]");
        }

        let id = image_index;
        self.set_images_count(image_index.0 + 1)?;
        info!("Image saved, total images count: {}", image_index.0 + 1);
        Ok(id)
    }

    fn read_image(&self, image_index: ImageId) -> cyberpixie_core::Result<DeviceImage<'_, Self>> {
        let images_count = self.images_count()?;

        if image_index >= images_count {
            return Err(CyberpixieError::ImageNotFound);
        }

        let header = self.read_image_header(image_index)?;
        let image = Image {
            refresh_rate: header.refresh_rate,
            bytes: ImageReader::new(
                BlockReaderImpl::new(self, image_index),
                header.image_len as usize,
            ),
        };
        Ok(image)
    }

    fn clear_images(&self) -> cyberpixie_core::Result<()> {
        let images_count = self.images_count()?;

        info!("Deleting {images_count} images...");
        for image_index in 0..images_count.0 {
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
