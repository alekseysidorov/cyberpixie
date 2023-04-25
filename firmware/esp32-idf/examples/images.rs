use cyberpixie_core::{proto::types::Hertz, service::DeviceStorage};
use cyberpixie_esp32_idf::{storage::ImagesRegistry, DEFAULT_DEVICE_CONFIG};
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

const RAW_IMAGE: &[u8] = include_bytes!("../../../assets/nyan_cat_48.raw").as_slice();

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    ImagesRegistry::erase()?;
    let images = ImagesRegistry::new(DEFAULT_DEVICE_CONFIG);
    images.add_image(Hertz(50), RAW_IMAGE)?;
    Ok(())
}
