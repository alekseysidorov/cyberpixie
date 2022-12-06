use std::path::Path;

pub use cyberpixie_std_transport::{connect_to, create_service, display_err, TcpTransport};
use image::io::Reader;

pub fn convert_image_to_raw(path: impl AsRef<Path>) -> anyhow::Result<(usize, Vec<u8>)> {
    let image = Reader::open(path)?.decode()?.to_rgb8();
    let width = image.width() as usize;

    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }

    Ok((width, raw))
}
