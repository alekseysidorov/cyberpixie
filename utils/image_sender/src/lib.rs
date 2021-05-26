#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use std::{
    fmt::Display,
    io,
    net::{SocketAddr, TcpStream},
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use cyberpixie_proto::{Hertz, PacketData, Service, Transport};
use image::io::Reader;

mod tcp_transport;

const TIMEOUT: Duration = Duration::from_secs(15);

fn connect_to(addr: &SocketAddr) -> anyhow::Result<TcpStream> {
    log::debug!("Connecting to the {}", addr);
    let stream = TcpStream::connect_timeout(addr, TIMEOUT)?;
    log::debug!("Connected");

    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.set_nodelay(true).ok();

    Ok(stream)
}

fn service_impl(addr: SocketAddr) -> anyhow::Result<Service<tcp_transport::TransportImpl, 512>> {
    let stream = connect_to(&addr)?;
    let transport = tcp_transport::TransportImpl::new(addr, stream);

    Ok(Service::new(transport))
}

pub fn convert_image_to_raw(path: impl AsRef<Path>) -> anyhow::Result<(usize, Vec<u8>)> {
    let image = Reader::open(path)?.decode()?.to_rgb8();
    let width = image.width() as usize;

    log::debug!("dimensions: {}, {}", image.width(), image.height());

    let mut raw = Vec::with_capacity(image.len() * 3);
    for rgb in image.pixels() {
        raw.push(rgb[0]);
        raw.push(rgb[1]);
        raw.push(rgb[2]);
    }

    Ok((width, raw))
}

fn display_err(err: impl Display) -> anyhow::Error {
    anyhow::format_err!("{}", err)
}

pub fn send_image(
    strip_len: usize,
    refresh_rate: Hertz,
    raw: Vec<u8>,
    to: SocketAddr,
) -> anyhow::Result<()> {
    let mut service = service_impl(to)?;

    let index = service
        .add_image(to, refresh_rate, strip_len, raw.into_iter())?
        .map_err(display_err)?;
    log::info!("Image loaded into the device {} with index {}", to, index);
    Ok(())
}

pub fn send_clear_images(to: SocketAddr) -> anyhow::Result<()> {
    let mut service = service_impl(to)?;

    service.clear_images(to)?.map_err(display_err)?;
    log::trace!("Sent images clear command to {}", to);
    Ok(())
}

pub fn send_show_image(index: usize, to: SocketAddr) -> anyhow::Result<()> {
    let mut service = service_impl(to)?;

    service.show_image(to, index)?.map_err(display_err)?;
    log::trace!("Showing image at {} on device {}", index, to);
    Ok(())
}

pub fn send_firmware_info(to: SocketAddr) -> anyhow::Result<()> {
    let mut service = service_impl(to)?;

    let info = service.request_firmware_info(to)?.map_err(display_err)?;
    log::info!("Got {:#?} from {}", info, to);
    Ok(())
}

pub fn run_transport_example(to: SocketAddr) -> anyhow::Result<()> {
    let stream = connect_to(&to)?;

    let mut transport = tcp_transport::TransportImpl::new(to, stream);
    let lines = spawn_stdin_channel();
    loop {
        match transport.poll_next_packet() {
            Ok(packet) => match packet.data {
                PacketData::Payload(payload) => {
                    for byte in payload {
                        eprint!("{}", byte as char);
                    }
                    transport.confirm_packet(packet.address)?;
                }
                PacketData::Confirmed => unreachable!(),
            },
            Err(nb::Error::WouldBlock) => {}
            Err(nb::Error::Other(err)) => return Err(err),
        };

        if let Ok(next_line) = lines.try_recv() {
            for data in next_line.as_bytes().chunks(256) {
                transport.send_packet(data.iter().copied(), to)?;
                nb::block!(transport.poll_for_confirmation(to))?;
            }
        };
    }
}

fn spawn_stdin_channel() -> Receiver<String> {
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
}
