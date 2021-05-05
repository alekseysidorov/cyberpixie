use embedded_hal::serial;

use crate::{
    adapter::{Adapter, ReadPart, WriterPart},
    parser::CommandResponse,
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
        self.init(config)?;
        Ok(self.adapter.into_parts())
    }

    fn init(&mut self, config: SoftApConfig<'_>) -> crate::Result<()> {
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

        Ok(())
    }
}

#[derive(Debug)]
pub enum Event<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    Connected {
        link_id: usize,
    },
    Closed {
        link_id: usize,
    },
    DataAvailable {
        link_id: usize,
        reader: DataReader<'a, Rx>,
    },
}

impl<Rx> ReadPart<Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    pub fn poll_data(&mut self) -> nb::Result<Event<'_, Rx>, Error> {
        let response =
            CommandResponse::parse(&self.buf).map(|(remainder, event)| (remainder.len(), event));
        if let Some((remaining_bytes, response)) = response {
            if remaining_bytes == 0 {
                self.buf.clear();
            } else {
                let pos = self.buf.len() - remaining_bytes;
                // FIXME Reduce complexity of the such kind operations.
                for to in 0..remaining_bytes {
                    let from = pos + to;
                    self.buf[to] = self.buf[from];
                }
                self.buf.truncate(remaining_bytes);
            }

            let event = match response {
                CommandResponse::Connected { link_id } => Event::Connected { link_id },
                CommandResponse::Closed { link_id } => Event::Closed { link_id },
                CommandResponse::DataAvailable { link_id, size } => Event::DataAvailable {
                    link_id,
                    reader: DataReader {
                        bytes_remaining: size,
                        read_pos: 0,
                        reader: self,
                    },
                },
            };

            return Ok(event);
        }

        self.read_bytes()
            .map_err(|inner| inner.map(|_| Error::Read))?;
        Err(nb::Error::WouldBlock)
    }
}

#[derive(Debug)]
pub struct DataReader<'a, Rx> {
    bytes_remaining: usize,
    read_pos: usize,
    reader: &'a mut ReadPart<Rx>,
}

impl<'a, Rx> Iterator for DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // We have received the all necessary bytes and should move the buffer to receive
            // the next pieces of data.
            if self.bytes_remaining == 0 {
                if self.read_pos > 0 {
                    // FIXME Reduce complexity of the such kind operations.
                    for from in self.read_pos..self.reader.buf.len() {
                        let to = from - self.read_pos;
                        self.reader.buf[to] = self.reader.buf[from];
                    }
                    self.reader
                        .buf
                        .truncate(self.reader.buf.len() - self.read_pos);
                    self.read_pos = 0;
                }
                return None;
            }
            // Try to get byte from the reader buffer.
            if self.read_pos < self.reader.buf.len() {
                let byte = self.reader.buf[self.read_pos];
                self.read_pos += 1;
                self.bytes_remaining -= 1;
                return Some(byte);
            }
            // At this point, we know that we have received the all bytes from the reader's buffer,
            // and thus we can safely clear it.
            if self.reader.buf.is_full() {
                self.reader.buf.clear();
                self.read_pos = 0;
            }

            // We need to wait for the next bytes batch.
            match self.reader.read_bytes() {
                Ok(_) | Err(nb::Error::WouldBlock) => continue,
                Err(e) => panic!("Panic in the iterator: {:?}", e),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.bytes_remaining, Some(self.bytes_remaining))
    }
}

impl<'a, Rx> ExactSizeIterator for DataReader<'a, Rx> where Rx: serial::Read<u8> + 'static {}

impl<'a, Rx> Drop for DataReader<'a, Rx>
// where
    // Rx: serial::Read<u8> + 'static,
{
    fn drop(&mut self) {
        todo!()
    }
}

 // FIXME Reduce complexity of this operation.
fn truncate_buf(buf: &mut [u8], at: usize) {
    
}
