use core::{
    cell::{Ref, RefCell},
    mem::MaybeUninit,
};

use cyberpixie::{leds::RGB8, proto::Hertz, ImagesRepository};
use embedded_sdmmc::{Block, BlockDevice, BlockIdx};
use endian_codec::{DecodeLE, EncodeLE, PackedSize};

use self::types::{Header, ImageDescriptor};

mod types;

/// The maximum amount of stored images.
pub const MAX_IMAGES_COUNT: usize = 60;

const BLOCK_SIZE: usize = 512;

struct ImageStorageInner<B> {
    device: B,
    block: HeaderBlock,
}

pub struct ImagesStorage<B> {
    inner: RefCell<ImageStorageInner<B>>,
}

impl<B> ImagesStorage<B>
where
    B: BlockDevice + 'static,
{
    pub fn open(device: B) -> Result<Self, B::Error> {
        let repository = Self {
            inner: RefCell::new(ImageStorageInner {
                device,
                block: HeaderBlock::empty(),
            }),
        };
        repository.inner.borrow_mut().get_or_init()?;
        Ok(repository)
    }
}

impl<B> ImageStorageInner<B>
where
    B: BlockDevice + 'static,
{
    /// This block is used to determine if the image repository has been initialized.
    /// If this block contains the `INIT_MSG` the repository is successfully initialized
    /// before.
    const INIT_BLOCK: BlockIdx = BlockIdx(0);
    /// The message should be presented in the `INIT_BLOCK` if this repository
    /// is has been initialized.
    const INIT_MSG: &'static [u8] = b"POI_STORAGE";
    /// This block contains the repository header.
    const HEADER_BLOCK: BlockIdx = BlockIdx(10);

    pub fn reset(&mut self) -> Result<&mut Self, B::Error> {
        self.init()?;
        Ok(self)
    }

    fn get_or_init(&mut self) -> Result<(), B::Error> {
        self.device
            .read(&mut self.block.inner, Self::INIT_BLOCK, "Load INIT block")?;

        if !self.block.inner[0].contents.starts_with(Self::INIT_MSG) {
            self.init()?;
        } else {
            self.device.read(
                &mut self.block.inner,
                Self::HEADER_BLOCK,
                "Load HEADER block",
            )?;
        }

        Ok(())
    }

    fn init(&mut self) -> Result<(), B::Error> {
        // Write INIT message to the first block.
        self.block.inner[0].contents[0..Self::INIT_MSG.len()].copy_from_slice(Self::INIT_MSG);
        self.device.write(&self.block.inner, Self::INIT_BLOCK)?;
        // Create and write a new header_block block of the images repository.
        self.block.set_header(Header {
            version: Header::VERSION,
            images_count: 0,
            vacant_block: (Self::HEADER_BLOCK.0 + 1) as u16,
        });
        self.device.write(&self.block.inner, Self::HEADER_BLOCK)?;
        Ok(())
    }

    fn count(&self) -> usize {
        self.block.header().images_count as usize
    }

    fn image_descriptor_at(&self, index: usize) -> ImageDescriptor {
        assert!(
            index < self.count(),
            "An attempt to read an image at an index greater than the total images count."
        );

        let descriptor_pos = Header::PACKED_LEN + index * ImageDescriptor::PACKED_LEN;
        ImageDescriptor::decode_from_le_bytes(&self.block.inner[0][descriptor_pos..])
    }

    fn add_image<I>(&mut self, data: I, refresh_rate: Hertz) -> Result<usize, B::Error>
    where
        I: Iterator<Item = RGB8>,
    {
        assert!(self.count() < MAX_IMAGES_COUNT);

        let mut header = self.block.header();

        // Sequentially write image bytes into the appropriate blocks.
        let (image_len, vacant_block) = {
            let bytes = data
                .map(|c| core::array::IntoIter::new([c.r, c.g, c.b]))
                .flatten();

            write_bytes(
                &mut self.device,
                bytes,
                BlockIdx(header.vacant_block as u32),
            )?
        };

        // Create a new image descriptor and add it to the header block.
        let descriptor = ImageDescriptor {
            block_number: header.vacant_block,
            image_len: image_len as u16,
            refresh_rate: refresh_rate.0,
        };
        let descriptor_pos =
            Header::PACKED_LEN + header.images_count as usize * ImageDescriptor::PACKED_LEN;
        descriptor.encode_as_le_bytes(self.block.inner[0][descriptor_pos..].as_mut());

        // Refresh header values.
        header.vacant_block = vacant_block.0 as u16;
        header.images_count += 1;
        let images_count = header.images_count;
        self.block.set_header(header);

        // Store updated header block.
        self.device.write(&self.block.inner, Self::HEADER_BLOCK)?;
        Ok(images_count as usize)
    }
}

struct HeaderBlock {
    inner: [Block; 1],
}

impl HeaderBlock {
    fn empty() -> Self {
        Self {
            inner: [Block::default()],
        }
    }

    fn header(&self) -> Header {
        Header::decode_from_le_bytes(self.inner[0].contents[0..].as_ref())
    }

    fn set_header(&mut self, header: Header) {
        header.encode_as_le_bytes(self.inner[0].contents[0..].as_mut())
    }
}

fn write_bytes<B, I>(
    device: &mut B,
    data: I,
    mut block_index: BlockIdx,
) -> Result<(usize, BlockIdx), B::Error>
where
    B: BlockDevice,
    I: Iterator<Item = u8>,
{
    let mut blocks = [Block {
        contents: unsafe { unitialized_block_content() },
    }];
    let mut i = 0;
    let mut c = 0;

    for byte in data {
        blocks[0][i] = byte;
        i += 1;
        c += 1;
        // If the current block is filled just flush it to the block device.
        if i == BLOCK_SIZE {
            device.write(&blocks, block_index)?;
            i = 0;
            block_index.0 += 1;
        }
    }
    // Special case for the last block.
    if i > 0 {
        // Fill the rest of the block with zeroes to prevent garbage.
        for j in i..BLOCK_SIZE {
            blocks[0][j] = 0;
        }
        device.write(&blocks, block_index)?;
        block_index.0 += 1;
    }

    Ok((c, block_index))
}

pub struct ReadImageIter<'a, B> {
    device: Ref<'a, B>,
    buf: [Block; 1],
    block_idx: BlockIdx,
    current_byte_in_block: usize,
    remaining_bytes: usize,
}

impl<'a, B: BlockDevice> Clone for ReadImageIter<'a, B> {
    fn clone(&self) -> Self {
        Self {
            device: Ref::clone(&self.device),
            buf: self.buf.clone(),
            block_idx: self.block_idx,
            current_byte_in_block: self.current_byte_in_block,
            remaining_bytes: self.remaining_bytes,
        }
    }
}

impl<'a, B: BlockDevice> ReadImageIter<'a, B> {
    fn new(device: Ref<'a, B>, block_idx: BlockIdx, bytes_to_read: usize) -> Self {
        assert!(
            bytes_to_read % 3 == 0,
            "Bytes amount to read must be a multiple of 3."
        );

        Self {
            device,
            buf: [Block {
                contents: unsafe { unitialized_block_content() },
            }],
            block_idx,
            current_byte_in_block: 0,
            remaining_bytes: bytes_to_read,
        }
    }

    fn block_data(&self) -> &[u8] {
        &self.buf[0][..]
    }
}

impl<'a, B: BlockDevice> Iterator for ReadImageIter<'a, B> {
    type Item = RGB8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_bytes == 0 {
            return None;
        }

        let mut color_bytes = [0_u8; 3];
        for color in &mut color_bytes {
            // In this case, we should read the next block from the device.
            if self.current_byte_in_block == 0 {
                self.device
                    .read(
                        &mut self.buf,
                        self.block_idx,
                        "Read block with the image content.",
                    )
                    .unwrap();
                // Move the cursor to the next block.
                self.block_idx.0 += 1;
            }

            let data = self.block_data();
            *color = data[self.current_byte_in_block];

            self.current_byte_in_block = (self.current_byte_in_block + 1) % BLOCK_SIZE;
            self.remaining_bytes -= 1;
        }

        Some(RGB8 {
            r: color_bytes[0],
            g: color_bytes[1],
            b: color_bytes[2],
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let image_len = self.remaining_bytes / 3;
        (image_len, Some(image_len))
    }
}

impl<'a, B: BlockDevice> ExactSizeIterator for ReadImageIter<'a, B> {}

impl<B> ImagesRepository for ImagesStorage<B>
where
    B: BlockDevice + 'static,
{
    type Error = B::Error;

    const MAX_COUNT: usize = MAX_IMAGES_COUNT;

    type ImagePixels<'b> = ReadImageIter<'b, B>;

    fn add_image<I>(&self, data: I, refresh_rate: Hertz) -> Result<usize, Self::Error>
    where
        I: Iterator<Item = RGB8>,
    {
        self.inner.borrow_mut().add_image(data, refresh_rate)
    }

    fn read_image(&self, index: usize) -> (Hertz, ReadImageIter<'_, B>) {
        let inner = self.inner.borrow();

        let descriptor = inner.image_descriptor_at(index);

        let refresh_rate = Hertz::from(descriptor.refresh_rate);
        let read_iter = ReadImageIter::new(
            Ref::map(inner, |inner| &inner.device),
            BlockIdx(descriptor.block_number as u32),
            descriptor.image_len as usize,
        );
        (refresh_rate, read_iter)
    }

    fn count(&self) -> usize {
        self.inner.borrow().block.header().images_count as usize
    }

    fn clear(&self) -> Result<(), Self::Error> {
        self.inner.borrow_mut().reset().map(drop)
    }
}

unsafe fn unitialized_block_content() -> [u8; 512] {
    let content: MaybeUninit<[u8; 512]> = MaybeUninit::uninit();
    content.assume_init()
}
