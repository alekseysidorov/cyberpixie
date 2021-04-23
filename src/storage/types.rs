use embedded_sdmmc::{Block, BlockDevice, BlockIdx};
use endian_codec::{DecodeLE, EncodeLE, PackedSize};
use gd32vf103xx_hal::time::Hertz;
use smart_leds::RGB8;

#[derive(Debug)]
pub struct ImagesCollection {
    block: Block,
}

#[derive(Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct Header {
    pub version: u16,
    pub images_count: u16,
    pub vacant_block: u16,
}

#[derive(Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct ImageDescriptor {
    block_number: u16,
    image_len: u16,
    refresh_rate: u32,
}

impl ImagesCollection {
    const VERSION: u16 = 1;

    pub(crate) fn new() -> Self {
        let mut collection = Self {
            block: Block::new(),
        };
        collection.set_header(Header {
            version: Self::VERSION,
            images_count: 0,
            vacant_block: 1,
        });
        collection
    }

    pub fn load<B: BlockDevice>(device: &mut B) -> Result<Self, B::Error> {
        let mut blocks = [Block::new()];
        device.read(blocks.as_mut(), BlockIdx(0), "Load block device")?;
        // FIXME It can eat a lot of memory.
        Ok(Self {
            block: blocks[0].clone(),
        })
    }

    pub fn save<B: BlockDevice>(&mut self, device: &mut B) -> Result<(), B::Error> {
        // FIXME It can eat a lot of memory.
        let blocks = [self.block.clone()];
        device.write(&blocks, BlockIdx(0))
    }

    pub fn header(&self) -> Header {
        Header::decode_from_le_bytes(self.block.contents[0..].as_ref())
    }

    pub fn add_image<B, I>(
        &mut self,
        device: &mut B,
        data: I,
        refresh_rate: Hertz,
    ) -> Result<(), B::Error>
    where
        B: BlockDevice,
        I: Iterator<Item = RGB8>,
    {
        let mut header = self.header();

        let bytes = data
            .map(|c| core::array::IntoIter::new([c.r, c.g, c.b]))
            .flatten();
        let (image_len, vacant_block) =
            write_bytes(device, bytes, BlockIdx(header.vacant_block as u32))?;

        let descriptor = ImageDescriptor {
            block_number: header.vacant_block,
            image_len: image_len as u16,
            refresh_rate: refresh_rate.0,
        };
        descriptor.encode_as_le_bytes(
            self.block
                [Header::PACKED_LEN + header.images_count as usize * ImageDescriptor::PACKED_LEN..]
                .as_mut(),
        );

        header.vacant_block = vacant_block.0 as u16;
        header.images_count += 1;
        self.set_header(header);

        self.save(device)
    }

    fn set_header(&mut self, header: Header) {
        header.encode_as_le_bytes(self.block.contents[0..].as_mut())
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
