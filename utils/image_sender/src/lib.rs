use std::{
    fmt::Display,
    io::Write,
    net::{TcpStream, ToSocketAddrs},
    path::Path,
};

use cyberpixie_proto::{
    types::{AddImage, MessageHeader},
    write_message_header, MAX_HEADER_LEN,
};
use image::io::Reader;

pub fn convert_image_to_raw(path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
    let image = Reader::open(path)?.decode()?.to_rgb8();

    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }

    Ok(raw)
}

pub fn send_image<T: ToSocketAddrs + Display + Copy>(
    strip_len: u16,
    refresh_rate: u32,
    raw: Vec<u8>,
    to: T,
) -> anyhow::Result<()> {
    let mut header_buf = vec![0_u8; MAX_HEADER_LEN];
    let msg = MessageHeader::AddImage(AddImage {
        refresh_rate,
        strip_len,
    });

    let total_len = write_message_header(&mut header_buf, &msg, raw.len())
        .map_err(|e| anyhow::format_err!("Unable to write message header: {:?}", e))?;
    header_buf.truncate(total_len);

    let mut stream = TcpStream::connect(to)?;
    stream.write_all(&header_buf)?;
    stream.write_all(&raw)?;

    log::trace!("Sent image to {}: {:?}", to, msg);
    Ok(())
}

pub fn send_clear_images<T: ToSocketAddrs + Display + Copy>(to: T) -> anyhow::Result<()> {
    let mut header_buf = vec![0_u8; MAX_HEADER_LEN];
    let msg = MessageHeader::ClearImages;

    let total_len = write_message_header(&mut header_buf, &msg, 0)
        .map_err(|e| anyhow::format_err!("Unable to write message header: {:?}", e))?;
    header_buf.truncate(total_len);

    let mut stream = TcpStream::connect(to)?;
    stream.write_all(&header_buf)?;

    log::trace!("Sent reset cmd to {}: {:?}", to, msg);
    Ok(())
}
