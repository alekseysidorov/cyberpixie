use core::fmt::Write;

use embedded_hal::serial;
use heapless::Vec;

use crate::{
    error::{Error, Result},
    ADAPTER_BUF_CAPACITY,
};

pub type RawResponse<'a> = core::result::Result<&'a [u8], &'a [u8]>;

const NEWLINE: &[u8] = b"\r\n";

#[derive(Debug)]
pub struct Adapter<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    Rx::Error: core::fmt::Debug,
    Tx::Error: core::fmt::Debug,
{
    pub(crate) reader: ReadPart<Rx>,
    pub(crate) writer: WritePart<Tx>,
    cmd_read_finished: bool,
}

impl<Rx, Tx> Adapter<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
    Rx::Error: core::fmt::Debug,
    Tx::Error: core::fmt::Debug,
{
    pub fn new(rx: Rx, tx: Tx) -> Result<Self, Rx::Error, Tx::Error> {
        let mut adapter = Self {
            reader: ReadPart {
                buf: Vec::default(),
                rx,
            },
            writer: WritePart { tx },
            cmd_read_finished: false,
        };
        adapter.init()?;
        Ok(adapter)
    }

    fn init(&mut self) -> Result<(), Rx::Error, Tx::Error> {
        // FIXME: It is ok to receive errors like "framing" during the reset procedure.
        self.reset().ok();
        // Workaround to catch the framing errors.
        for _ in 0..50 {
            self.send_at_command_str(b"ATE1").ok();
        }
        self.reader.buf.clear();

        self.disable_echo()?;
        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Rx::Error, Tx::Error> {
        self.send_command_impl(b"AT+RST")?;
        self.read_until(ReadyCondition)?;

        Ok(())
    }

    pub fn send_at_command_str(
        &mut self,
        cmd: impl AsRef<[u8]>,
    ) -> Result<RawResponse<'_>, Rx::Error, Tx::Error> {
        self.send_command_impl(cmd.as_ref())?;
        self.read_until(OkCondition)
    }

    pub fn send_at_command_fmt(
        &mut self,
        args: core::fmt::Arguments,
    ) -> Result<RawResponse<'_>, Rx::Error, Tx::Error> {
        self.writer.write_fmt(args).map_err(|_| Error::Format)?;
        self.writer.write_bytes(NEWLINE).map_err(Error::Write)?;

        self.read_until(OkCondition)
    }

    fn disable_echo(&mut self) -> Result<(), Rx::Error, Tx::Error> {
        self.send_at_command_str(b"ATE0").map(drop)
    }

    fn send_command_impl(&mut self, cmd: &[u8]) -> Result<(), Rx::Error, Tx::Error> {
        self.writer.write_bytes(cmd).map_err(Error::Write)?;
        self.writer.write_bytes(NEWLINE).map_err(Error::Write)
    }

    pub(crate) fn clear_reader_buf(&mut self) {
        self.cmd_read_finished = false;
        // Safety: `u8` is aprimitive type and doesn't have drop implementation so we can just
        // modify the buffer length.
        unsafe {
            self.reader.buf.set_len(0);
        }
    }

    fn read_until<'a, C>(&'a mut self, condition: C) -> Result<C::Output, Rx::Error, Tx::Error>
    where
        C: Condition<'a>,
    {
        if self.cmd_read_finished {
            self.clear_reader_buf();
        }

        loop {
            match self.reader.read_bytes() {
                Ok(_) => {
                    if self.reader.buf.is_full() {
                        return Err(Error::BufferFull);
                    }
                }
                Err(nb::Error::WouldBlock) => {}
                Err(nb::Error::Other(err)) => {
                    self.cmd_read_finished = true;
                    return Err(Error::Read(err));
                }
            };

            if condition.is_performed(&self.reader.buf) {
                self.cmd_read_finished = true;
                break;
            }
        }

        Ok(condition.output(&self.reader.buf))
    }
}

trait Condition<'a>: Copy + Clone {
    type Output: 'a;

    fn is_performed(self, buf: &[u8]) -> bool;

    fn output(self, buf: &'a [u8]) -> Self::Output;
}

#[derive(Clone, Copy)]
struct ReadyCondition;

impl ReadyCondition {
    const MSG: &'static [u8] = b"ready\r\n";
}

impl<'a> Condition<'a> for ReadyCondition {
    type Output = &'a [u8];

    fn is_performed(self, buf: &[u8]) -> bool {
        buf.ends_with(Self::MSG)
    }

    fn output(self, buf: &'a [u8]) -> Self::Output {
        &buf[0..buf.len() - Self::MSG.len()]
    }
}

#[derive(Clone, Copy)]
struct OkCondition;

impl OkCondition {
    const OK: &'static [u8] = b"OK\r\n";
    const ERROR: &'static [u8] = b"ERROR\r\n";
}

impl<'a> Condition<'a> for OkCondition {
    type Output = core::result::Result<&'a [u8], &'a [u8]>;

    fn is_performed(self, buf: &[u8]) -> bool {
        buf.ends_with(Self::OK) || buf.ends_with(Self::ERROR)
    }

    fn output(self, buf: &'a [u8]) -> Self::Output {
        if buf.ends_with(Self::OK) {
            Ok(&buf[0..buf.len() - Self::OK.len()])
        } else {
            Err(&buf[0..buf.len() - Self::ERROR.len()])
        }
    }
}

#[derive(Debug)]
pub struct ReadPart<Rx> {
    rx: Rx,
    pub buf: Vec<u8, ADAPTER_BUF_CAPACITY>,
}

impl<Rx> ReadPart<Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    pub(crate) fn read_bytes(&mut self) -> nb::Result<(), Rx::Error> {
        loop {
            if self.buf.is_full() {
                return Err(nb::Error::WouldBlock);
            }

            let byte = self.rx.read()?;
            // Safety: we have already checked if this buffer is full,
            // a couple of lines above.
            unsafe {
                self.buf.push_unchecked(byte);
            }
            stdio_serial::dprint!("{}", byte as char);
        }
    }
}

#[derive(Debug)]
pub struct WritePart<Tx> {
    tx: Tx,
}

impl<Tx> WritePart<Tx>
where
    Tx: serial::Write<u8> + 'static,
    Tx::Error: core::fmt::Debug,
{
    fn write_fmt(&mut self, args: core::fmt::Arguments) -> core::fmt::Result {
        let writer = &mut self.tx as &mut (dyn serial::Write<u8, Error = Tx::Error> + 'static);
        writer.write_fmt(args)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> core::result::Result<(), Tx::Error> {
        for byte in bytes.iter() {
            stdio_serial::dprint!("{}", *byte as char);
            nb::block!(self.tx.write(*byte))?;
        }
        Ok(())
    }
}
