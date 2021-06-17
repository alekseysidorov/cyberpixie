use core::{fmt::Debug, format_args};

use embedded_hal::serial;

use crate::{adapter::Adapter, Error, TcpSocket};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub channel: u8,
    pub mode: u8,
}

impl<'a> SoftApConfig<'a> {
    pub fn start<Rx, Tx>(
        mut self,
        mut adapter: Adapter<Rx, Tx>,
    ) -> crate::Result<TcpSocket<Rx, Tx>, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        self.init(&mut adapter)?;
        Ok(TcpSocket::new(adapter))
    }

    fn init<Rx, Tx>(
        &mut self,
        adapter: &mut Adapter<Rx, Tx>,
    ) -> crate::Result<(), Rx::Error, Tx::Error>
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

        // Enable multiple connections.
        adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPMUX",
                msg: "Unable to enable multiple connections",
            })?;

        // Setup a TCP server.
        adapter
            .send_at_command_str("AT+CIPSERVER=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSERVER",
                msg: "Unable to setup a TCP server",
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
        adapter.clear_reader_buf();

        Ok(())
    }
}
