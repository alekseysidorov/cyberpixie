use core::fmt::Write;

use arrayvec::ArrayVec;
use embedded_hal::serial;

use crate::{
    error::{Error, Result},
    ADAPTER_BUF_CAPACITY,
};

const NEWLINE: &[u8] = b"\r\n";

pub struct Adapter<Rx, Tx> {
    reader: ReadPart<Rx>,
    writer: WriterPart<Tx>,
    cmd_read_finished: bool,
}

impl<Rx, Tx> Adapter<Rx, Tx>
where
    Rx: serial::Read<u8> + 'static,
    Tx: serial::Write<u8> + 'static,
{
    pub fn new(rx: Rx, tx: Tx) -> Result<Self> {
        let mut adapter = Self {
            reader: ReadPart {
                buf: ArrayVec::default(),
                rx,
            },
            writer: WriterPart { tx },
            cmd_read_finished: false,
        };
        adapter.reset()?;
        adapter.disable_echo()?;
        Ok(adapter)
    }

    pub fn reset(&mut self) -> Result<()> {
        self.send_command_impl(b"AT+RST")?;

        self.read_until(ReadyCondition)?;
        Ok(())
    }

    pub fn send_at_command_str(
        &mut self,
        cmd: impl AsRef<[u8]>,
    ) -> Result<core::result::Result<&'_ [u8], &'_ [u8]>> {
        self.send_command_impl(cmd.as_ref())?;

        self.read_until(OkCondition)
    }

    pub fn send_at_command_fmt(
        &mut self,
        args: core::fmt::Arguments,
    ) -> Result<core::result::Result<&'_ [u8], &'_ [u8]>> {
        self.writer.write_fmt(args).map_err(|_| Error::Write)?;
        self.writer.write_bytes(NEWLINE).map_err(|_| Error::Write)?;

        self.read_until(OkCondition)
    }

    pub(crate) fn into_parts(mut self) -> (ReadPart<Rx>, WriterPart<Tx>) {
        self.reader.buf.clear();        
        (self.reader, self.writer)
    }

    fn disable_echo(&mut self) -> Result<()> {
        self.send_at_command_str(b"ATE0").map(drop)
    }

    fn send_command_impl(&mut self, cmd: &[u8]) -> Result<()> {
        self.writer.write_bytes(cmd).map_err(|_| Error::Write)?;
        self.writer.write_bytes(NEWLINE).map_err(|_| Error::Write)
    }

    fn read_until<'a, C>(&'a mut self, condition: C) -> Result<C::Output>
    where
        C: Condition<'a>,
    {
        if self.cmd_read_finished {
            self.cmd_read_finished = false;
            self.reader.buf.clear();
        }

        loop {
            match self.reader.read_bytes() {
                Ok(_) => {
                    if self.reader.buf.is_full() {
                        return Err(Error::BufferFull);
                    }
                }
                Err(nb::Error::WouldBlock) => {}
                Err(_) => return Err(Error::Read),
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

pub struct ReadPart<Rx> {
    rx: Rx,
    pub(crate) buf: ArrayVec<u8, ADAPTER_BUF_CAPACITY>,
}

impl<Rx> ReadPart<Rx>
where
    Rx: serial::Read<u8> + 'static,
{
    pub(crate) fn read_bytes(&mut self) -> nb::Result<(), Rx::Error> {
        while self.buf.remaining_capacity() > 0 {
            self.buf.push(self.rx.read()?);
        }
        Ok(())
    }
}

pub struct WriterPart<Tx> {
    tx: Tx,
}

impl<Tx> WriterPart<Tx>
where
    Tx: serial::Write<u8> + 'static,
{
    fn write_fmt(&mut self, args: core::fmt::Arguments) -> core::fmt::Result {
        let writer = &mut self.tx as &mut (dyn serial::Write<u8, Error = Tx::Error> + 'static);
        writer.write_fmt(args)
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> core::result::Result<(), Tx::Error> {
        for byte in bytes.iter() {
            nb::block!(self.tx.write(*byte))?;
        }
        Ok(())
    }
}
