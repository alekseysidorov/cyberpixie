use cyberpixie_app::{
    core::{
        io::image_reader::ImageLines,
        proto::types::{Hertz, ImageId},
        ExactSizeRead,
    },
    Configuration, Storage,
};
use cyberpixie_embedded_storage::{
    test_utils::{leaked_buf, MemoryBackend},
    MemoryLayout, StorageImpl,
};
use embedded_io::blocking::Read;

fn init_storage() -> StorageImpl<MemoryBackend> {
    StorageImpl::init(
        Configuration::default(),
        MemoryBackend::default(),
        MemoryLayout {
            base: 0x9000,
            size: 0xFFFFF,
        },
        leaked_buf(512),
    )
    .unwrap()
}

#[test]
fn test_config_read_write() {
    let mut storage = init_storage();

    let expected_config = Configuration {
        strip_len: 32,
        current_image: None,
    };
    storage.set_config(expected_config).unwrap();

    let actual_config = storage.config().unwrap();
    assert_eq!(actual_config, expected_config);
}

fn read_image(storage: &mut StorageImpl<MemoryBackend>, id: ImageId) -> (Hertz, Vec<u8>) {
    let mut image = storage.read_image(id).unwrap();
    let mut buf = vec![0_u8; image.bytes.bytes_remaining()];
    image.bytes.read_exact(&mut buf).unwrap();
    (image.refresh_rate, buf)
}

#[tokio::test]
async fn image_read_write_simple() {
    let mut storage = init_storage();

    let image_data_1 = [1_u8; 72];
    let image_data_2 = [2_u8; 24 * 3 * 20];

    // Add a first image
    storage.add_image(Hertz(500), &image_data_1[..]).await.unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(1));

    // Read a first image
    let (rate, data) = read_image(&mut storage, ImageId(0));
    assert_eq!(rate, Hertz(500));
    assert_eq!(data, image_data_1);

    // Add a second image
    storage.add_image(Hertz(42), &image_data_2[..]).await.unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(2));

    // Read a first image again
    let (rate, data) = read_image(&mut storage, ImageId(0));
    assert_eq!(rate, Hertz(500));
    assert_eq!(data, image_data_1);

    // Read a second image
    let (rate, data) = read_image(&mut storage, ImageId(1));
    assert_eq!(rate, Hertz(42));
    assert_eq!(data, image_data_2);
}

#[tokio::test]
async fn image_read_write_clear() {
    let mut storage = init_storage();

    let image_data_1 = [1_u8; 72];

    // Add an images
    storage.add_image(Hertz(500), &image_data_1[..]).await.unwrap();
    storage.add_image(Hertz(42), &image_data_1[..]).await.unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(2));
    // Clear images
    storage.clear_images().unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(0));
    // Add an image again

    let image_data_2 = [2_u8; 24 * 3 * 20];
    storage.add_image(Hertz(48), &image_data_2[..]).await.unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(1));

    // Read it
    let (rate, data) = read_image(&mut storage, ImageId(0));
    assert_eq!(rate, Hertz(48));
    assert_eq!(data, image_data_2);
}

#[tokio::test]
async fn test_image_lines_cycle_nyan_cat() {
    let mut storage = init_storage();

    // Read an image
    let image = image::load_from_memory(include_bytes!("../../../assets/nyan_cat_48.png"))
        .unwrap()
        .to_rgb8();
    // Convert image to raw bytes
    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.extend(rgb.0);
    }
    // Add image.
    storage.add_image(Hertz(500), &raw[..]).await.unwrap();
    // Read image line by line.
    let image = storage.read_image(ImageId(0)).unwrap();
    let mut lines = ImageLines::new(image, 48, vec![0_u8; 512]);

    let first_line: Vec<_> = lines.next_line().unwrap().collect();
    // Render a lot of lines
    for _ in 1..360 {
        let _line = lines.next_line().unwrap();
    }
    // After several cycles we should return back to the first image line
    let line: Vec<_> = lines.next_line().unwrap().collect();
    assert_eq!(first_line, line);
    // Check that the next line is not equal the first one
    let line: Vec<_> = lines.next_line().unwrap().collect();
    assert_ne!(first_line, line);    
}
