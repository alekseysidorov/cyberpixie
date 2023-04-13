use std::fmt::Display;

use cyberpixie_core::{
    proto::types::{Hertz, ImageId},
    service::{Config, DeviceImage, DeviceStorage, Image},
    storage::BlockReader,
    Error as CyberpixieError, ExactSizeRead,
};
use embedded_svc::storage::RawStorage;
use esp_idf_sys::EspError;
use log::info;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{BLOCK_SIZE, STORAGE};

#[derive(Debug, Serialize, Deserialize)]
struct ImageHeader {
    image_len: u32,
    refresh_rate: Hertz,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ImagesRegistry {
    default_config: Config,
}

impl ImagesRegistry {
    pub fn new(default_config: Config) -> Self {
        Self { default_config }
    }

    pub fn erase() -> Result<(), EspError> {
        // esp-idf lacks of the erase wrapper, so we have to use unsafe code in order to erase images registry.
        unsafe {
            let code = esp_idf_sys::nvs_flash_erase();
            esp_idf_sys::esp!(code)
        }
    }

    fn set<T>(&self, name: &str, value: &T) -> cyberpixie_core::Result<()>
    where
        T: Serialize,
    {
        let mut buf = vec![0_u8; BLOCK_SIZE];

        postcard::to_slice(value, &mut buf).map_err(CyberpixieError::encode)?;
        self.set_raw(name, &buf)
            .map_err(CyberpixieError::storage_write)?;
        Ok(())
    }

    fn get<T>(&self, name: &str) -> Result<Option<T>, CyberpixieError>
    where
        T: DeserializeOwned,
    {
        let mut buf = vec![0_u8; BLOCK_SIZE];

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
        Ok(config.unwrap_or(self.default_config))
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

        let value = self
            .get("img.current")
            .map_err(CyberpixieError::storage_read)?;
        Ok(value.unwrap_or(Some(ImageId(0))))
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
        let mut buf = vec![0_u8; BLOCK_SIZE];

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
                vec![0_u8; BLOCK_SIZE],
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

pub type ImageReader<'a> = cyberpixie_core::storage::ImageReader<BlockReaderImpl<'a>, Vec<u8>>;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct BlockReadError(pub EspError);

impl Display for BlockReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for BlockReadError {}

impl embedded_io::Error for BlockReadError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl From<EspError> for BlockReadError {
    fn from(value: EspError) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct BlockReaderImpl<'a> {
    registry: &'a ImagesRegistry,
    image_index: ImageId,
}

impl<'a> BlockReaderImpl<'a> {
    pub fn new(registry: &'a ImagesRegistry, image_index: ImageId) -> Self {
        Self {
            registry,
            image_index,
        }
    }
}

impl<'a> embedded_io::Io for BlockReaderImpl<'a> {
    type Error = BlockReadError;
}

impl<'a, const N: usize> BlockReader<N> for BlockReaderImpl<'a> {
    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<(), Self::Error> {
        let idx = self.image_index.0;

        log::trace!("Filling block {block} [0..{}]", buf.len(),);
        self.registry
            .get_raw(&format!("img.{idx}.block.{block}"), buf)?;
        Ok(())
    }
}
