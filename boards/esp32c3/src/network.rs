//! Network driver for the embassy-net stack
//! TODO move to separate crate.

use cyberpixie_app::{
    network::asynch::{NetworkSocket, NetworkStack},
    CyberpixieError, CyberpixieResult,
};
use embassy_net::{tcp::TcpSocket, IpListenEndpoint, Stack};
use esp_wifi::wifi::WifiDevice;

pub struct NetworkStackImpl {
    stack: &'static Stack<WifiDevice<'static>>,
}

pub struct NetworkSocketImpl {
    rx: [u8; 1024],
    tx: [u8; 1024],
    stack: &'static Stack<WifiDevice<'static>>,
}

impl NetworkSocket for NetworkSocketImpl {
    type ConnectionError = embassy_net::tcp::Error;
    type Connection<'a> = TcpSocket<'a>;

    async fn accept(&mut self, port: u16) -> CyberpixieResult<Self::Connection<'_>> {
        let mut socket = TcpSocket::new(self.stack, &mut self.rx, &mut self.tx);
        socket.set_timeout(Some(embassy_net::SmolDuration::from_secs(30)));

        socket
            .accept(IpListenEndpoint { addr: None, port })
            .await
            .map_err(CyberpixieError::network)?;
        Ok(socket)
    }
}

impl NetworkStackImpl {
    /// Creates a new network driver implementation.
    pub fn new(stack: &'static Stack<WifiDevice<'static>>) -> Self {
        Self { stack }
    }
}

impl NetworkStack for NetworkStackImpl {
    type Socket<'a> = NetworkSocketImpl where Self: 'a;

    fn socket(&mut self) -> Self::Socket<'_> {
        NetworkSocketImpl {
            rx: [0_u8; 1024],
            tx: [0_u8; 1024],
            stack: self.stack,
        }
    }
}
