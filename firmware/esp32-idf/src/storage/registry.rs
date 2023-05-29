use std::fmt::Display;

use cyberpixie_app::{
    core::{
        proto::types::{Hertz, ImageId},
        storage::{BlockReader, Image},
        Error as CyberpixieError, ExactSizeRead,
    },
    Configuration, Storage, CyberpixieResult,
};
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
    default_config: Configuration,
}

impl ImagesRegistry {
    #[must_use]
    pub const fn new(default_config: Configuration) -> Self {
        Self { default_config }
    }

    // Erases images registry memory.
    //
    // esp-idf lacks of the erase wrapper, so we have to use unsafe code in order to erase images registry.
    pub fn erase() -> Result<(), EspError> {
        let _guard = STORAGE.lock().unwrap();
        // # Safety
        //
        // We protect storage access by mutex, so we guarantee that there is no concurrent
        // access to the storage partition.
        #[allow(unsafe_code)]
        unsafe {
            let code = esp_idf_sys::nvs_flash_erase();
            esp_idf_sys::esp!(code)
        }
    }

    fn set<T>(name: &str, value: &T) -> CyberpixieResult<()>
    where
        T: Serialize,
    {
        let mut buf = vec![0_u8; BLOCK_SIZE];

        postcard::to_slice(value, &mut buf).map_err(CyberpixieError::encode)?;
        Self::set_raw(name, &buf).map_err(CyberpixieError::storage_write)?;
        Ok(())
    }

    fn get<T>(name: &str) -> Result<Option<T>, CyberpixieError>
    where
        T: DeserializeOwned,
    {
        let mut buf = vec![0_u8; BLOCK_SIZE];

        let bytes = Self::get_raw(name, &mut buf).map_err(CyberpixieError::storage_read)?;

        bytes
            .map(|buf| postcard::from_bytes(buf))
            .transpose()
            .map_err(CyberpixieError::decode)
    }

    fn set_raw(name: &str, buf: &[u8]) -> CyberpixieResult<bool> {
        STORAGE
            .lock()
            .unwrap()
            .set_raw(name, buf)
            .map_err(CyberpixieError::storage_write)
    }

    fn get_raw<'a>(name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, EspError> {
        STORAGE.lock().unwrap().get_raw(name, buf)
    }

    fn remove(name: &str) -> CyberpixieResult<bool> {
        info!("Removing '{name}' entry...");
        STORAGE
            .lock()
            .unwrap()
            .remove(name)
            .map_err(CyberpixieError::storage_write)
    }

    fn set_images_count(count: u16) -> Result<(), CyberpixieError> {
        Self::set("img.count", &count).map_err(CyberpixieError::storage_write)
    }

    fn read_image_header(image_index: ImageId) -> CyberpixieResult<ImageHeader> {
        Self::get(&format!("img.{image_index}.header"))?.ok_or(CyberpixieError::StorageRead)
    }
}

pub type ImageReader<'a> = cyberpixie_app::core::storage::ImageReader<BlockReaderImpl<'a>, Vec<u8>>;

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
    _registry: &'a ImagesRegistry,
    image_index: ImageId,
}

impl<'a> BlockReaderImpl<'a> {
    pub const fn new(_registry: &'a ImagesRegistry, image_index: ImageId) -> Self {
        #[allow(clippy::used_underscore_binding)]
        Self {
            _registry,
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
        ImagesRegistry::get_raw(&format!("img.{idx}.block.{block}"), buf)?;
        Ok(())
    }
}

impl Storage for ImagesRegistry {
    type ImageReader<'a> = ImageReader<'a>
    where
        Self: 'a;

    fn config(&self) -> CyberpixieResult<Configuration> {
        let config = Self::get("config")?;
        Ok(config.unwrap_or(self.default_config))
    }

    fn set_config(&mut self, new: Configuration) -> CyberpixieResult<()> {
        // TODO Implement checks.
        Self::set("config", &new).map_err(CyberpixieError::storage_write)
    }

    fn add_image<R: ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        mut image: R,
    ) -> CyberpixieResult<ImageId> {
        let image_index = self.images_count()?;

        // Save image header.
        let header = ImageHeader {
            image_len: image.bytes_remaining() as u32,
            refresh_rate,
        };
        Self::set(&format!("img.{image_index}.header"), &header)?;
        info!("Saving image with header: {header:?}");

        // Save image content.
        let mut buf = vec![0_u8; BLOCK_SIZE];

        let blocks = image.bytes_remaining() / BLOCK_SIZE;
        for block in 0..=blocks {
            let to = std::cmp::min(image.bytes_remaining(), BLOCK_SIZE);
            image
                .read_exact(&mut buf[0..to])
                .map_err(CyberpixieError::storage_write)?;

            Self::set_raw(&format!("img.{image_index}.block.{block}"), &buf[0..to])?;
            info!("Write block {block} -> [0..{to}]");
        }

        let id = image_index;
        Self::set_images_count(image_index.0 + 1)?;
        info!("Image saved, total images count: {}", image_index.0 + 1);
        Ok(id)
    }

    fn read_image(
        &self,
        image_id: ImageId,
    ) -> CyberpixieResult<cyberpixie_app::ImageReader<'_, Self>> {
        let images_count = self.images_count()?;

        if image_id >= images_count {
            return Err(CyberpixieError::ImageNotFound);
        }

        let header = Self::read_image_header(image_id)?;
        let image = Image {
            refresh_rate: header.refresh_rate,
            bytes: ImageReader::new(
                BlockReaderImpl::new(self, image_id),
                header.image_len as usize,
                vec![0_u8; BLOCK_SIZE],
            ),
        };
        Ok(image)
    }

    fn images_count(&self) -> CyberpixieResult<ImageId> {
        Self::get("img.count")
            .map(Option::unwrap_or_default)
            .map_err(CyberpixieError::storage_read)
    }

    fn clear_images(&mut self) -> CyberpixieResult<()> {
        let images_count = self.images_count()?;

        info!("Deleting {images_count} images...");
        for image_index in 0..images_count.0 {
            let header = Self::read_image_header(ImageId(image_index))?;
            // Remove image blocks.
            let blocks_count = header.image_len as usize / BLOCK_SIZE;
            for block in 0..=blocks_count {
                Self::remove(&format!("img.{image_index}.block.{block}"))?;
            }
            // Remove image header.
            Self::remove(&format!("img.{image_index}.header"))?;
        }
        // Reset images counter.
        Self::set_images_count(0)?;

        Ok(())
    }
}
