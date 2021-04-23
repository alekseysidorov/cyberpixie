use self::types::{Header, ImageDescriptor};
use embedded_sdmmc::{Block, BlockDevice, BlockIdx};
use endian_codec::{DecodeLE, EncodeLE, PackedSize};
use gd32vf103xx_hal::time::Hertz;
use smart_leds::RGB8;

mod types;

/// The maximum amount of stored images.
pub const MAX_IMAGES_COUNT: usize = 60;

pub struct ImagesRepository<'a, B> {
    device: &'a mut B,
    block: HeaderBlock,
}

impl<'a, B> ImagesRepository<'a, B>
where
    B: BlockDevice,
{
    /// This block is used to determine if the image repository has been initialized. 
    /// If this block contains the `INIT_MSG` the repository is successfully initialized
    /// before.
    const INIT_BLOCK: BlockIdx = BlockIdx(0);
    /// The message should be presented in the `INIT_BLOCK` if this repository 
    /// is has been initialized.
    const INIT_MSG: &'static [u8] = b"POI_STORAGE";
    /// This block contains the repository header.
    const HEADER_BLOCK: BlockIdx = BlockIdx(1);

    pub fn open(device: &'a mut B) -> Result<Self, B::Error> {
        let (block, device) = Self::get_or_init(device)?;
        Ok(Self { device, block })
    }

    pub fn add_image<I>(&mut self, data: I, refresh_rate: Hertz) -> Result<(), B::Error>
    where
        B: BlockDevice,
        I: Iterator<Item = RGB8>,
    {
        let mut header = self.block.header();

        // Sequentially write image bytes into the appropriate blocks.
        let (image_len, vacant_block) = {
            let bytes = data
                .map(|c| core::array::IntoIter::new([c.r, c.g, c.b]))
                .flatten();
            write_bytes(self.device, bytes, BlockIdx(header.vacant_block as u32))?
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
        self.block.set_header(header);

        // Store updated header block.
        self.device.write(&self.block.inner, Self::HEADER_BLOCK)
    }

    pub fn count(&self) -> usize {
        self.block.header().images_count as usize
    }

    fn get_or_init(device: &'a mut B) -> Result<(HeaderBlock, &'a mut B), B::Error> {
        let mut header_block = HeaderBlock {
            inner: [Block::new()],
        };

        device.read(&mut header_block.inner, Self::INIT_BLOCK, "Load INIT block")?;
        if !header_block.inner[0].contents.starts_with(Self::INIT_MSG) {
            // Write INIT message to the first block.
            header_block.inner[0].contents[0..Self::INIT_MSG.len()].copy_from_slice(Self::INIT_MSG);
            device.write(&header_block.inner, Self::INIT_BLOCK)?;
            // Create and write a new header_block block of the images repository.
            header_block.set_header(Header {
                version: Header::VERSION,
                images_count: 0,
                vacant_block: (Self::HEADER_BLOCK.0 + 1) as u16,
            });
            device.write(&header_block.inner, Self::HEADER_BLOCK)?;
        } else {
            device.read(
                &mut header_block.inner,
                Self::HEADER_BLOCK,
                "Load HEADER block",
            )?;
        }

        Ok((header_block, device))
    }
}

struct HeaderBlock {
    inner: [Block; 1],
}

impl HeaderBlock {
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
    let mut blocks = [Block::new()];
    let mut i = 0;
    let mut c = 0;

    for byte in data {
        blocks[0][i] = byte;
        i += 1;
        c += 1;
        // If the current block is filled just flush it to the block device.
        if i == 512 {
            device.write(&blocks, block_index)?;
            i = 0;
            block_index.0 += 1;
        }
    }
    // Special case for the last block.
    if i > 0 {
        // Fill the rest of the block with zeroes to prevent garbage.
        for j in i..512 {
            blocks[0][j] = 0;
        }
        device.write(&blocks, block_index)?;
        block_index.0 += 1;
    }

    Ok((c, block_index))
}
