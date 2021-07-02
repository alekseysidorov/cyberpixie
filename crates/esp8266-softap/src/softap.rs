use core::{fmt::Debug, format_args};

use embedded_hal::serial;
use no_std_net::{IpAddr, SocketAddr};
use simple_clock::SimpleClock;

use crate::{adapter::Adapter, parser::CifsrResponse, TcpSocket};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub channel: u8,
    pub mode: u8,
}

impl<'a> SoftApConfig<'a> {
    pub fn start<Rx, Tx, C>(
        self,
        mut adapter: Adapter<Rx, Tx, C>,
    ) -> crate::Result<TcpSocket<Rx, Tx, C>>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        let address = self.init(&mut adapter)?;
        Ok(TcpSocket::new(adapter, address))
    }

    fn init<Rx, Tx, C>(&self, adapter: &mut Adapter<Rx, Tx, C>) -> crate::Result<IpAddr>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        // Enable SoftAP+Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=3")?
            .expect("Malformed command");

        // Start SoftAP.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWSAP=\"{}\",\"{}\",{},{}",
                self.ssid, self.password, self.channel, self.mode,
            ))?
            .expect("Malformed command");

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .expect("Malformed command");

        // Setup a TCP server.
        adapter
            .send_at_command_str("AT+CIPSERVER=1,333")?
            .expect("Malformed command");

        // Get assigned SoftAP address.
        let raw_resp = adapter
            .send_at_command_fmt(format_args!("AT+CIFSR"))?
            .expect("Malformed command");
        let ap_addr = CifsrResponse::parse(raw_resp).unwrap().1.ap_ip.unwrap();

        adapter.clear_reader_buf();
        Ok(ap_addr)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct JoinApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub link_id: usize,
    pub address: SocketAddr,
}

impl<'a> JoinApConfig<'a> {
    pub fn join<Rx, Tx, C>(
        self,
        mut adapter: Adapter<Rx, Tx, C>,
    ) -> crate::Result<TcpSocket<Rx, Tx, C>>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        let address = self.init(&mut adapter)?;
        Ok(TcpSocket::new(adapter, address))
    }

    fn init<Rx, Tx, C>(&self, adapter: &mut Adapter<Rx, Tx, C>) -> crate::Result<IpAddr>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        C: SimpleClock,
    {
        // Enable Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=1")?
            .expect("Malformed command");

        // Join the given access point.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWJAP=\"{}\",\"{}\"",
                self.ssid, self.password,
            ))?
            .expect("Malformed command");

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .expect("Malformed command");

        // Setup a TCP server.
        adapter
            .send_at_command_str("AT+CIPSERVER=1,334")?
            .expect("Malformed command");

        // Establish TCP connection with the given address.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CIPSTART={},\"TCP\",\"{}\",{}",
                self.link_id,
                self.address.ip(),
                self.address.port(),
            ))?
            .expect("Malformed command");

        // Get assigned SoftAP address.
        let raw_resp = adapter
            .send_at_command_fmt(format_args!("AT+CIFSR"))?
            .expect("Malformed command");
        let ap_addr = CifsrResponse::parse(raw_resp).unwrap().1.sta_ip.unwrap();

        adapter.clear_reader_buf();
        Ok(ap_addr)
    }
}
