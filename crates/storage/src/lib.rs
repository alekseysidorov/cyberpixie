//! Cyberpixie storage implementation
//!
//! This crate implements generic storage on top of the [`embedded_storage`] traits.

#![cfg_attr(not(any(feature = "std", test)), no_std)]
// Linter configuration
#![warn(unsafe_code, missing_copy_implementations)]
#![warn(clippy::pedantic)]
#![warn(clippy::use_self)]
// Too many false positives.
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation
)]

use cyberpixie_app::{
    core::{
        proto::types::{Hertz, ImageId},
        storage::Image,
        ExactSizeRead,
    },
    Configuration, CyberpixieError, CyberpixieResult, ImageReader,
};
use embedded_io::{
    blocking::{Read, Seek},
    Io, SeekFrom,
};
use endian_codec::{DecodeLE, EncodeLE, PackedSize};
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "std", test))]
pub mod test_utils;

/// Storage offset length in bytes.
const OFFSET_LEN: usize = core::mem::size_of::<u32>();

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(any(feature = "std", test), derive(Debug))]
struct Metadata {
    current_image: Option<ImageId>,
}

/// The storage header block.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(feature = "std", test), derive(Debug))]
struct Header {
    /// Storage layout version.
    version: u16,
    /// LED strip length.
    strip_len: u16,
    /// Saved images count.
    images_count: ImageId,
    /// Additional metadata, may differ depending on the storage version.
    metadata: Metadata,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: 1,
            strip_len: 24,
            images_count: ImageId(0),
            metadata: Metadata::default(),
        }
    }
}

impl From<Header> for Configuration {
    fn from(header: Header) -> Self {
        Self {
            strip_len: header.strip_len,
            current_image: header.metadata.current_image,
        }
    }
}

impl Header {
    /// Header block size.
    const BLOCK_SIZE: usize = 512;
    /// Header block location.
    const LOCATION: u32 = 0;

    /// Updates header with the specified configuration and returns `true` if config has breaking changes.
    ///
    /// # Panics
    ///
    /// - if the specified `current_image_index` is not suitable with the saved images count.
    fn update(&mut self, config: Configuration) -> bool {
        assert!(config.current_image <= Some(self.images_count));

        let has_breaking_changes = self.strip_len != config.strip_len;

        self.strip_len = config.strip_len;
        self.metadata.current_image = config.current_image;
        has_breaking_changes
    }

    /// Reads and decodes header block.
    fn read<T: embedded_storage::Storage>(
        backend: &mut T,
        layout: MemoryLayout,
        buf: &mut [u8],
    ) -> CyberpixieResult<Self> {
        backend
            .read(Self::location_offset(layout), buf)
            .map_err(|_| CyberpixieError::StorageRead)?;

        postcard::from_bytes(buf).map_err(CyberpixieError::decode)
    }

    /// Writes a header block back to the embedded storage memory.
    fn write<T: embedded_storage::Storage>(
        &self,
        backend: &mut T,
        layout: MemoryLayout,
        buf: &mut [u8],
    ) -> CyberpixieResult<()> {
        postcard::to_slice(self, buf).map_err(CyberpixieError::storage_write)?;

        backend
            .write(Self::location_offset(layout), &buf[0..Self::BLOCK_SIZE])
            .map_err(|_| CyberpixieError::StorageWrite)
    }

    /// Calculates the offset of the header block.
    #[inline]
    fn location_offset(layout: MemoryLayout) -> u32 {
        layout.base + Self::LOCATION
    }
}

/// Picture location pair.
///
/// This structure uses to read information about the current picture location and the next one,
/// and thus calculate the current picture length in bytes.
#[derive(Clone, Copy, PartialEq, PackedSize, EncodeLE, DecodeLE)]
#[cfg_attr(any(feature = "std", test), derive(Debug))]
struct PictureLocation {
    /// Current picture location.
    current: u32,
    /// Next picture location.
    next: u32,
}

impl PictureLocation {
    /// Pictures registry block size.
    const BLOCK_SIZE: usize = 512;
    /// Pictures registry block location.
    const LOCATION: usize = Header::BLOCK_SIZE;

    /// Creates a location object for the first picture.
    fn first() -> Self {
        Self {
            current: 0,
            next: (Self::LOCATION + Self::BLOCK_SIZE) as u32,
        }
    }

    /// Reads and decodes picture location.
    fn read<T: embedded_storage::Storage>(
        image_id: ImageId,
        backend: &mut T,
        layout: MemoryLayout,
        buf: &mut [u8],
    ) -> CyberpixieResult<Self> {
        Self::location_offset(layout, image_id);
        // Limit read buffer to the offset pair length
        let bytes = &mut buf[0..OFFSET_LEN * 2];
        // Read picture location from the embedded storage.
        backend
            .read(Self::location_offset(layout, image_id), bytes)
            .map_err(|_| CyberpixieError::StorageRead)?;
        Ok(Self::decode_from_le_bytes(bytes))
    }

    /// Writes a picture location back to the embedded storage memory.
    ///
    /// # Important notice
    ///
    /// The `next` offset of latest location pair should point to the vacant entry for a next image.
    fn write<T: embedded_storage::Storage>(
        self,
        image_id: ImageId,
        backend: &mut T,
        layout: MemoryLayout,
        buf: &mut [u8],
    ) -> CyberpixieResult<()> {
        // Limit write buffer to the offset pair length and write location pair to it.
        let bytes = &mut buf[0..OFFSET_LEN * 2];
        self.encode_as_le_bytes(bytes);
        // Write picture location to the embedded storage.
        backend
            .write(Self::location_offset(layout, image_id), bytes)
            .map_err(|_| CyberpixieError::StorageWrite)
    }

    /// Calculates the offset of the picture location with the specified ID.
    fn location_offset(layout: MemoryLayout, image_id: ImageId) -> u32 {
        layout.base + (Self::LOCATION + OFFSET_LEN * usize::from(image_id.0)) as u32
    }
}

/// Storage memory layout
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MemoryLayout {
    /// Flash address of The base address of the partition beginning.
    pub base: u32,
    /// The partition size.
    pub size: u32,
}

/// Cyberpixie storage which contains a device configuration parameters and
/// saved pictures.
///
/// This storage uses a very simple linear layout:
///
/// TBD
pub struct StorageImpl<T: embedded_storage::Storage> {
    backend: T,
    // Storage memory layout.
    layout: MemoryLayout,
    // Internal buffer to read and write data.
    buf: &'static mut [u8],
}

impl<T: embedded_storage::Storage> StorageImpl<T> {
    /// Max count of pictures which can be stored.
    const MAX_PICTURES_NUM: u16 =
        (PictureLocation::BLOCK_SIZE / core::mem::size_of::<u32>() - 1) as u16;

    /// Opens a new Cyberpixie storage.
    ///
    /// # Panics
    ///
    /// - if the given buffer length less that the 512 bytes.
    pub fn open(
        backend: T,
        layout: MemoryLayout,
        buf: &'static mut [u8],
    ) -> CyberpixieResult<Self> {
        assert!(buf.len() >= Header::BLOCK_SIZE);

        Ok(Self {
            backend,
            layout,
            buf,
        })
    }

    /// Initializes a new Cyberpixie storage
    ///
    /// Unlike the [`Self::open`] method, it formats the store and initializes it with the specified configuration.
    ///
    /// # Panics
    ///
    /// - if the given buffer length less that the 512 bytes.
    pub fn init(
        config: Configuration,
        mut backend: T,
        layout: MemoryLayout,
        buf: &'static mut [u8],
    ) -> CyberpixieResult<Self> {
        // Initialize storage memory with a new header block.
        let new_header = Header {
            strip_len: config.strip_len,
            ..Header::default()
        };
        new_header.write(&mut backend, layout, buf)?;

        Self::open(backend, layout, buf)
    }

    /// Returns a vacant location for a new picture.
    fn vacant_location(&mut self, images_count: ImageId) -> CyberpixieResult<PictureLocation> {
        if images_count.0 == 0 {
            Ok(PictureLocation::first())
        } else {
            let last_image = ImageId(images_count.0 - 1);
            PictureLocation::read(last_image, &mut self.backend, self.layout, self.buf)
        }
    }
}

/// Picture file content.
pub struct PictureFile<'a, T: embedded_storage::ReadStorage> {
    backend: &'a mut T,
    // Begin of file offset
    begin_offset: u32,
    // End of file offset
    end_offset: u32,
    // Current read pos
    read_pos: u32,
}

impl<'a, T: embedded_storage::ReadStorage> Io for PictureFile<'a, T> {
    type Error = CyberpixieError;
}

impl<'a, T: embedded_storage::ReadStorage> Read for PictureFile<'a, T> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let amount = core::cmp::min(buf.len(), self.bytes_remaining());
        // Read amount of bytes from the storage backend to the given buffer.
        let buf = &mut buf[0..amount];
        self.backend
            .read(self.read_pos, buf)
            .map_err(|_| CyberpixieError::StorageRead)?;
        // Update read position
        self.read_pos += amount as u32;

        Ok(amount)
    }
}

impl<'a, T: embedded_storage::ReadStorage> Seek for PictureFile<'a, T> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        // Compute a new image read position
        self.read_pos = match pos {
            SeekFrom::Start(pos) => self.begin_offset + pos as u32,
            // In this project, we only have to read an image from the beginning,
            // so we don't need to implement the whole seek functionality
            SeekFrom::Current(pos) => self.read_pos.saturating_add_signed(pos as i32),
            SeekFrom::End(pos) => self.end_offset.saturating_add_signed(pos as i32),
        };
        Ok(u64::from(self.read_pos))
    }
}

impl<'a, T: embedded_storage::ReadStorage> ExactSizeRead for PictureFile<'a, T> {
    #[inline]
    fn bytes_remaining(&self) -> usize {
        (self.end_offset - self.read_pos) as usize
    }
}

impl<T: embedded_storage::Storage + Send + 'static> cyberpixie_app::Storage for StorageImpl<T> {
    type ImageRead<'a> = PictureFile<'a, T>;

    fn config(&mut self) -> CyberpixieResult<Configuration> {
        let header = Header::read(&mut self.backend, self.layout, self.buf)?;
        Ok(header.into())
    }

    fn set_config(&mut self, config: Configuration) -> CyberpixieResult<()> {
        let mut header = Header::read(&mut self.backend, self.layout, self.buf)?;
        let has_breaking_changes = header.update(config);
        header.write(&mut self.backend, self.layout, self.buf)?;
        // Clear images if configuration has breaking changes.
        if has_breaking_changes {
            self.clear_images()?;
        }
        Ok(())
    }

    fn add_image<R: Read + ExactSizeRead>(
        &mut self,
        refresh_rate: Hertz,
        mut image: R,
    ) -> CyberpixieResult<ImageId> {
        let mut header = Header::read(&mut self.backend, self.layout, self.buf)?;

        // Check preconditions
        if header.images_count.0 >= Self::MAX_PICTURES_NUM {
            return Err(CyberpixieError::ImageRepositoryIsFull);
        }
        // FIXME check image len

        // Next image start offset
        let image_id = header.images_count;
        let last_picture = self.vacant_location(image_id)?;
        let mut offset = last_picture.next;

        // Write the refresh rate.
        {
            let buf = &mut self.buf[0..Hertz::PACKED_LEN];
            refresh_rate.encode_as_le_bytes(buf);
            self.backend
                .write(offset, buf)
                .map_err(|_| CyberpixieError::StorageWrite)?;
            offset += buf.len() as u32;
        }

        // Write image bytes
        while !image.is_empty() {
            let bytes_read = image.read(self.buf).map_err(CyberpixieError::network)?;
            self.backend
                .write(offset, &self.buf[0..bytes_read])
                .map_err(|_| CyberpixieError::StorageWrite)?;
            offset += bytes_read as u32;
        }

        // Save new image location.
        PictureLocation {
            current: last_picture.next,
            next: offset,
        }
        .write(image_id, &mut self.backend, self.layout, self.buf)?;

        // Update storage header.
        header.images_count.0 += 1;
        header.write(&mut self.backend, self.layout, self.buf)?;

        Ok(image_id)
    }

    fn read_image(&mut self, image_id: ImageId) -> CyberpixieResult<ImageReader<'_, Self>> {
        // Check preconditions.
        if image_id >= self.images_count()? {
            return Err(CyberpixieError::ImageNotFound);
        }

        // Get picture location
        let location = PictureLocation::read(image_id, &mut self.backend, self.layout, self.buf)?;

        // Calculate picture file offsets.
        let mut begin_offset = location.current;
        let end_offset = location.next;
        // Read refresh rate
        let refresh_rate = {
            let buf = &mut self.buf[0..Hertz::PACKED_LEN];
            self.backend
                .read(location.current, buf)
                .map_err(|_| CyberpixieError::StorageRead)?;
            let rate = Hertz::decode_from_le_bytes(buf);
            begin_offset += buf.len() as u32;
            rate
        };

        // Return an image reader.
        Ok(Image {
            refresh_rate,
            bytes: PictureFile {
                backend: &mut self.backend,
                begin_offset,
                end_offset,
                read_pos: begin_offset,
            },
        })
    }

    fn images_count(&mut self) -> CyberpixieResult<ImageId> {
        let header = Header::read(&mut self.backend, self.layout, self.buf)?;
        Ok(header.images_count)
    }

    fn clear_images(&mut self) -> CyberpixieResult<()> {
        let mut header = Header::read(&mut self.backend, self.layout, self.buf)?;
        header.images_count = ImageId(0);
        header.metadata.current_image = None;
        header.write(&mut self.backend, self.layout, self.buf)
    }
}

#[cfg(test)]
mod tests {
    use cyberpixie_app::{core::proto::types::ImageId, Configuration};

    use crate::{
        test_utils::{leaked_buf, MemoryBackend},
        Header, MemoryLayout, Metadata, PictureLocation, StorageImpl,
    };

    impl PictureLocation {
        /// Returns a location pair for the next image.
        fn next_image(self, image_len: u32) -> Self {
            Self {
                current: self.next,
                next: self.next + image_len,
            }
        }
    }

    fn init_storage() -> StorageImpl<MemoryBackend> {
        StorageImpl::init(
            Configuration::default(),
            MemoryBackend::default(),
            MemoryLayout {
                base: 0x9000,
                size: 0xFFFFF,
            },
            leaked_buf(),
        )
        .unwrap()
    }

    #[test]
    fn test_init_storage() {
        let _storage = init_storage();
    }

    #[test]
    fn test_header_read_write() {
        let mut backend = MemoryBackend::default();
        let layout = MemoryLayout {
            base: 0,
            size: 0xFFFFF,
        };
        let buf = leaked_buf();

        let expected_header = Header {
            strip_len: 18,
            images_count: ImageId(4),
            metadata: Metadata {
                current_image: Some(ImageId(2)),
            },
            ..Header::default()
        };
        expected_header.write(&mut backend, layout, buf).unwrap();

        let actual_header = Header::read(&mut backend, layout, buf).unwrap();
        assert_eq!(actual_header, expected_header);
    }

    #[test]
    fn test_picture_location_read_write() {
        let mut backend = MemoryBackend::default();
        let layout = MemoryLayout {
            base: 0x8000,
            size: 0xFFFFF,
        };
        let buf = leaked_buf();

        let first_location = PictureLocation {
            current: 0,
            next: 1024,
        };
        first_location
            .write(ImageId(0), &mut backend, layout, buf)
            .unwrap();
        assert_eq!(
            first_location,
            PictureLocation::read(ImageId(0), &mut backend, layout, buf).unwrap()
        );

        let next_location = first_location.next_image(3072);
        next_location
            .write(ImageId(1), &mut backend, layout, buf)
            .unwrap();

        assert_eq!(
            first_location,
            PictureLocation::read(ImageId(0), &mut backend, layout, buf).unwrap()
        );
        assert_eq!(
            next_location,
            PictureLocation::read(ImageId(1), &mut backend, layout, buf).unwrap()
        );
    }
}
