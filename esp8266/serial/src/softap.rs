use embedded_hal::serial;
use moveslice::Moveslice;

use crate::{
    adapter::{Adapter, ReadPart, WriterPart},
    parser::NetworkEvent,
    Error,
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct SoftApConfig<'a> {
    pub ssid: &'a str,
    pub password: &'a str,
    pub channel: u8,
    pub mode: u8,
}

pub struct SoftAp<Rx, Tx> {
    adapter: Adapter<Rx, Tx>,
}

impl<Rx, Tx> SoftAp<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
{
    pub fn new(adapter: Adapter<Rx, Tx>) -> Self {
        Self { adapter }
    }

    pub fn start(
        mut self,
        config: SoftApConfig<'_>,
    ) -> crate::Result<(ReadPart<Rx>, WriterPart<Tx>)> {
        // Enable SoftAP+Station mode.
        self.adapter
            .send_at_command_str("AT+CWMODE=3")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWMODE",
                msg: "Unable to set Wifi mode",
            })?;

        // Enable multiple connections.
        self.adapter
            .send_at_command_str("AT+CIPMUX=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPMUX",
                msg: "Unable to enable multiple connections",
            })?;

        // Setup a TCP server.
        self.adapter
            .send_at_command_str("AT+CIPSERVER=1")?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSERVER",
                msg: "Unable to setup a TCP server",
            })?;

        // Start SoftAP.
        self.adapter
            .send_at_command_fmt(core::format_args!(
                "AT+CWSAP=\"{}\",\"{}\",{},{}",
                config.ssid,
                config.password,
                config.channel,
                config.mode,
            ))?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CWSAP",
                msg: "Incorrect soft AP configuration",
            })?;

        Ok(self.adapter.into_parts())
    }
}

impl<Rx> ReadPart<Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    pub fn poll_data(&mut self) -> nb::Result<NetworkEvent, Error> {
        if let Some((remainder, event)) = NetworkEvent::parse(&self.buf) {
            if remainder.is_empty() {
                self.buf.clear();
            } else {
                let pos = self.buf.len() - remainder.len();
                // FIXME Reduce complexity of the such kind operations.
                self.buf.moveslice(pos..self.buf.len(), 0);
            }
            return Ok(event);
        }

        self
            .read_bytes()
            .map_err(|inner| inner.map(|_| Error::Read))?;
        Err(nb::Error::WouldBlock)
    }
}
