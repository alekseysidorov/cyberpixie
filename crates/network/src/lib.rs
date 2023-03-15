use std::{
    fmt::Display,
    net::{SocketAddr, TcpStream},
    time::Duration,
};

pub use network_part::{Client, NetworkPart};

mod connection;
mod network_part;

const TIMEOUT: Duration = Duration::from_secs(120);

pub fn display_err(err: impl Display) -> anyhow::Error {
    anyhow::format_err!("{}", err)
}

pub fn connect_to(addr: &SocketAddr) -> std::io::Result<TcpStream> {
    log::debug!("Connecting to the {}", addr);
    let stream = TcpStream::connect_timeout(addr, TIMEOUT)?;
    log::debug!("Connected");

    stream.set_nodelay(true).ok();
    Ok(stream)
}

pub fn create_client(addr: SocketAddr) -> std::io::Result<Client> {
    let stream = connect_to(&addr)?;
    Client::connect(stream)
}
