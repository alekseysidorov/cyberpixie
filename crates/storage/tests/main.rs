use cyberpixie_app::{
    core::{
        proto::types::{Hertz, ImageId},
        ExactSizeRead,
    },
    Configuration, Storage,
};
use cyberpixie_embedded_storage::{
    test_utils::{leaked_buf, MemoryBackend},
    StorageImpl,
};
use embedded_io::blocking::Read;

fn init_storage() -> StorageImpl<MemoryBackend> {
    StorageImpl::init(
        Configuration::default(),
        MemoryBackend::default(),
        leaked_buf(),
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

#[test]
fn image_read_write_simple() {
    let mut storage = init_storage();

    let image_data_1 = [1_u8; 72];
    let image_data_2 = [2_u8; 24 * 3 * 20];

    // Add a first image
    storage.add_image(Hertz(500), &image_data_1[..]).unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(1));

    // Read a first image
    let (rate, data) = read_image(&mut storage, ImageId(0));
    assert_eq!(rate, Hertz(500));
    assert_eq!(data, image_data_1);

    // Add a second image
    storage.add_image(Hertz(42), &image_data_2[..]).unwrap();
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

#[test]
fn image_read_write_clear() {
    let mut storage = init_storage();

    let image_data_1 = [1_u8; 72];

    // Add an images
    storage.add_image(Hertz(500), &image_data_1[..]).unwrap();
    storage.add_image(Hertz(42), &image_data_1[..]).unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(2));
    // Clear images
    storage.clear_images().unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(0));
    // Add an image again

    let image_data_2 = [2_u8; 24 * 3 * 20];
    storage.add_image(Hertz(48), &image_data_2[..]).unwrap();
    assert_eq!(storage.images_count().unwrap(), ImageId(1));

    // Read it
    let (rate, data) = read_image(&mut storage, ImageId(0));
    assert_eq!(rate, Hertz(48));
    assert_eq!(data, image_data_2);
}
