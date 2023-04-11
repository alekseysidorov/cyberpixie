use cyberpixie_core::{
    proto::types::{Hertz, ImageId},
    service::DeviceStorage,
    ExactSizeRead,
};
use cyberpixie_esp32c3::storage::ImagesRegistry;
use embedded_io::blocking::Read;
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

const RAW_IMAGE: &[u8] = include_bytes!("../../../assets/nyan_cat_48.png").as_slice();

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let images = ImagesRegistry::new();
    images.set_config(&cyberpixie_core::service::Config { strip_len: 48 })?;

    images.clear_images()?;
    images.add_image(Hertz(1), RAW_IMAGE)?;

    let mut reader = images.read_image(ImageId(0))?;

    let mut buf = vec![0_u8; reader.bytes.bytes_remaining()];
    reader.bytes.read_exact(&mut buf)?;
    log::info!("{}", std::str::from_utf8(&buf).unwrap());

    Ok(())
}
