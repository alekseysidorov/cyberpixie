use core::{
    fmt::{self, Debug},
    format_args,
};

use embedded_hal::serial;
use heapless::Vec;

use crate::{
    adapter::{Adapter, CarretCondition, OkCondition, ReadPart},
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

impl<'a> SoftApConfig<'a> {
    pub fn start<Rx, Tx>(
        mut self,
        mut adapter: Adapter<Rx, Tx>,
    ) -> crate::Result<TcpStream<Rx, Tx>, Rx::Error, Tx::Error>
    where
        Rx: serial::Read<u8> + 'static,
        Tx: serial::Write<u8> + 'static,
        Rx::Error: core::fmt::Debug,
        Tx::Error: core::fmt::Debug,
    {
        self.init(&mut adapter)?;
        Ok(TcpStream { adapter })
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

pub struct TcpStream<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    Rx::Error: core::fmt::Debug,
    Tx::Error: core::fmt::Debug,
{
    adapter: Adapter<Rx, Tx>,
}

impl<Rx, Tx> TcpStream<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    Rx::Error: core::fmt::Debug,
    Tx::Error: core::fmt::Debug,
{
    pub fn from_raw(mut adapter: Adapter<Rx, Tx>) -> Self {
        adapter.reader.buf.clear();
        Self { adapter }
    }

    pub fn read_bytes(&mut self) -> nb::Result<(), Rx::Error> {
        self.adapter.reader.read_bytes()
    }

    pub fn poll_next_event(&mut self) -> nb::Result<Event<'_, Rx>, Rx::Error> {
        self.adapter.reader.poll_next_event()
    }

    pub fn send_packet_to_link<I>(
        &mut self,
        link_id: usize,
        bytes: I,
    ) -> crate::Result<(), Rx::Error, Tx::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let bytes_len = bytes.len();
        // TODO Implement sending of the whole bytes by splitting them into chunks.
        assert!(
            bytes_len < 2048,
            "Total packet size should not be greater than the 2048 bytes"
        );
        assert!(self.adapter.reader.buf.is_empty());

        self.adapter
            .write_command_fmt(format_args!("AT+CIPSEND={},{}", link_id, bytes_len))?;
        self.adapter.read_until(CarretCondition)?;

        for byte in bytes {
            nb::block!(self.adapter.writer.write_byte(byte)).map_err(Error::Write)?;
        }

        self.adapter
            .read_until(OkCondition)?
            .map_err(|_| Error::MalformedCommand {
                cmd: "CIPSEND",
                msg: "Incorrect usage of the CIPSEND (with link_id) command",
            })?;
        self.adapter.clear_reader_buf();
        Ok(())
    }
}

#[derive(Debug)]
pub enum Event<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
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
    Rx::Error: core::fmt::Debug,
{
    pub(crate) fn poll_next_event(&mut self) -> nb::Result<Event<'_, Rx>, Rx::Error> {
        let response =
            CommandResponse::parse(&self.buf).map(|(remainder, event)| (remainder.len(), event));

        if let Some((remaining_bytes, response)) = response {
            let pos = self.buf.len() - remaining_bytes;
            truncate_buf(&mut self.buf, pos);

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

        self.read_bytes()?;
        Err(nb::Error::WouldBlock)
    }
}

pub struct DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    pub(crate) bytes_remaining: usize,
    pub(crate) read_pos: usize,
    pub(crate) reader: &'a mut ReadPart<Rx>,
}

impl<'a, Rx> Debug for DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataReader")
            .field("bytes_remaining", &self.bytes_remaining)
            .field("read_pos", &self.read_pos)
            .field("buf", &self.reader.buf)
            .finish()
    }
}

impl<'a, Rx> DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    fn finish(&mut self) {
        if self.read_pos > 0 {
            truncate_buf(&mut self.reader.buf, self.read_pos);
            self.read_pos = 0;
        }
    }
}

impl<'a, Rx> Iterator for DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // We need to wait for the next bytes batch.
            match self.reader.read_bytes() {
                Ok(_) | Err(nb::Error::WouldBlock) => {}
                Err(e) => panic!("Panic in the iterator: {:?}", e),
            }
            // We have received the all necessary bytes and should move the buffer to receive
            // the next pieces of data.
            if self.bytes_remaining == 0 {
                self.finish();
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
                // Safety: `u8` is aprimitive type and doesn't have drop implementation so we can just
                // modify the buffer length.
                unsafe {
                    self.reader.buf.set_len(0);
                }
                self.read_pos = 0;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.bytes_remaining, Some(self.bytes_remaining))
    }
}

impl<'a, Rx> ExactSizeIterator for DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
}

impl<'a, Rx> Drop for DataReader<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    fn drop(&mut self) {
        // FIXME: Rethink data reader to enable this code.
        // In order to use the reader further, we must read all of the remaining bytes.
        // Otherwise, the reader will be in an inconsistent state.
        // for _ in self {}

        self.finish()
    }
}

// FIXME: Reduce complexity of this operation.
fn truncate_buf<const N: usize>(buf: &mut Vec<u8, N>, at: usize) {
    assert!(at <= buf.len());

    for from in at..buf.len() {
        let to = from - at;
        buf[to] = buf[from];
    }
    // Safety: `u8` is aprimitive type and doesn't have drop implementation so we can just
    // modify the buffer length.
    unsafe {
        buf.set_len(buf.len() - at);
    }
}
