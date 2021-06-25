use core::{fmt::Debug, format_args};

use embedded_hal::serial;
use no_std_net::{IpAddr, SocketAddr};

use crate::{adapter::Adapter, parser::CifsrResponse, Error, TcpSocket};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub channel: u8,
    pub mode: u8,
}

impl<'a> SoftApConfig<'a> {
    pub fn start<Rx, Tx>(
        self,
        mut adapter: Adapter<Rx, Tx>,
    ) -> crate::Result<TcpSocket<Rx, Tx>, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        let address = self.init(&mut adapter)?;
        Ok(TcpSocket::new(adapter, address))
    }

    fn init<Rx, Tx>(
        &self,
        adapter: &mut Adapter<Rx, Tx>,
    ) -> crate::Result<IpAddr, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,

        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        // Enable SoftAP+Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=3")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWMODE",
                msg: "Unable to set Wifi mode",
            })?;

        // Start SoftAP.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWSAP=\"{}\",\"{}\",{},{}",
                self.ssid, self.password, self.channel, self.mode,
            ))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWSAP",
                msg: "Incorrect soft AP configuration",
            })?;

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPMUX",
                msg: "Unable to enable multiple connections",
            })?;

        // Setup a TCP server.
        adapter
            .send_at_command_str("AT+CIPSERVER=1,333")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSERVER",
                msg: "Unable to setup a TCP server",
            })?;

        // Get assigned SoftAP address.
        let raw_resp = adapter
            .send_at_command_fmt(format_args!("AT+CIFSR"))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIFSR",
                msg: "Incorrect usage of the CIFSR command",
            })?;
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
    pub fn join<Rx, Tx>(
        self,
        mut adapter: Adapter<Rx, Tx>,
    ) -> crate::Result<TcpSocket<Rx, Tx>, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        let address = self.init(&mut adapter)?;
        Ok(TcpSocket::new(adapter, address))
    }

    fn init<Rx, Tx>(
        &self,
        adapter: &mut Adapter<Rx, Tx>,
    ) -> crate::Result<IpAddr, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,

        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        // Enable Station mode.
        adapter
            .send_at_command_str("AT+CWMODE=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWMODE",
                msg: "Unable to set Wifi mode",
            })?;

        // Join the given access point.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CWJAP=\"{}\",\"{}\"",
                self.ssid, self.password,
            ))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWJAP",
                msg: "Unable to join the selected access point.",
            })?;

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPMUX",
                msg: "Unable to enable multiple connections",
            })?;

        // Setup a TCP server.
        adapter
            .send_at_command_str("AT+CIPSERVER=1,334")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSERVER",
                msg: "Unable to setup a TCP server",
            })?;

        // Establish TCP connection with the given address.
        adapter
            .send_at_command_fmt(format_args!(
                "AT+CIPSTART={},\"TCP\",\"{}\",{}",
                self.link_id,
                self.address.ip(),
                self.address.port(),
            ))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSTART",
                msg: "Unable to connect with the specified host",
            })?;

        // Get assigned SoftAP address.
        let raw_resp = adapter
            .send_at_command_fmt(format_args!("AT+CIFSR"))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIFSR",
                msg: "Incorrect usage of the CIFSR command",
            })?;
        let ap_addr = CifsrResponse::parse(raw_resp).unwrap().1.sta_ip.unwrap();

        adapter.clear_reader_buf();
        Ok(ap_addr)
    }
}
