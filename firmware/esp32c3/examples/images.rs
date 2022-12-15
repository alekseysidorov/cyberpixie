use cyberpixie_esp32c3::storage::ImagesRegistry;
use cyberpixie_proto::{
    types::{Hertz, ImageId},
    ExactSizeRead,
};
use cyberpixie_storage::DeviceStorage;
use embedded_io::blocking::Read;
use esp_idf_svc::log::EspLogger;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys as _;

const BIG_TEXT: &[u8] = b"The standard Lorem Ipsum passage, used since the 1500s \
    Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
    tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, \
    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. \
    Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu \
    fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, \
    sunt in culpa qui officia deserunt mollit anim id est laborum. \
    \
    Section 1.10.32 of 'de Finibus Bonorum et Malorum', written by Cicero in 45 BC \
    Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque \
    laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi \
    architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas \
    sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione \
    voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, \
    consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et \
    dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum \
    exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi \
    consequatur? Quis autem vel eum iure reprehenderit qui in ea voluptate velit esse \
    quam nihil molestiae consequatur, vel illum qui dolorem eum fugiat quo voluptas nulla pariatur?";

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    EspLogger::initialize_default();

    let images = ImagesRegistry::new();

    images.clear_images()?;
    images.add_image(Hertz(50), BIG_TEXT)?;

    let mut reader = images.read_image(ImageId(0))?.unwrap();

    let mut buf = vec![0_u8; reader.bytes.bytes_remaining()];
    reader.bytes.read_exact(&mut buf)?;
    log::info!("{}", std::str::from_utf8(&buf).unwrap());

    Ok(())
}
