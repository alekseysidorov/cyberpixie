use embedded_sdmmc::BlockDevice;
use gd32vf103xx_hal::time::Hertz;
use smart_leds::RGB8;

use crate::uprintln;

pub use self::types::ImagesCollection;

mod types;

pub fn format(device: &mut impl BlockDevice) {
    let mut collection = ImagesCollection::new();
    uprintln!("Created a new collection: {:?}", collection.header());
    if let Err(e) = collection.save(device) {
        uprintln!("Format failed {:?}", e);
    }
}

pub fn read_header(device: &mut impl BlockDevice) {
    let collection = ImagesCollection::load(device).unwrap();
    uprintln!("Loaded header: {:?}", collection.header())
}

pub fn add_image(device: &mut impl BlockDevice, image: &[RGB8], refresh_rate: Hertz) {
    let mut collection = ImagesCollection::load(device).unwrap();
    if let Err(e) = collection.add_image(device, image.iter().copied(), refresh_rate) {
        uprintln!("Add image failed {:?}", e);
    }
}
