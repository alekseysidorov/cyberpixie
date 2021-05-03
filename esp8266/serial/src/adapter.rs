use arrayvec::ArrayVec;
use embedded_hal::serial;

pub const ADAPTER_BUF_CAPACITY: usize = 512;

#[derive(Debug)]
pub enum Error {
    Read,
    Write,
    BufferFull,
}

pub struct Adapter<Rx, Tx>
where
    Rx: serial::Read<u8>,
    Tx: serial::Write<u8>,
{
    rx: Rx,
    tx: Tx,
    buf: ArrayVec<u8, ADAPTER_BUF_CAPACITY>,
    read_finished: bool,
}

impl<Rx, Tx> Adapter<Rx, Tx>
where
    Rx: serial::Read<u8>,
    Tx: serial::Write<u8>,
{
    pub fn new(rx: Rx, tx: Tx) -> Result<Self, Error> {
        let mut adapter = Self {
            rx,
            tx,
            buf: ArrayVec::default(),
            read_finished: false,
        };
        adapter.reset()?;
        adapter.disable_echo()?;
        Ok(adapter)
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        self.send_command(b"AT+RST")?;
        self.read_until(ReadyCondition)?;

        Ok(())
    }

    pub fn send_at_command(&mut self, raw: &[u8]) -> Result<Result<&'_ [u8], &'_ [u8]>, Error> {
        self.send_command(raw)?;
        self.read_until(OkCondition)
    }

    fn disable_echo(&mut self) -> Result<(), Error> {
        self.send_command(b"ATE0")?;
        self.read_until(OkCondition)?.map_err(|_| Error::Read)?;

        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Tx::Error> {
        for byte in bytes.iter() {
            nb::block!(self.tx.write(*byte))?;
        }
        Ok(())
    }

    fn send_command(&mut self, cmd: &[u8]) -> Result<(), Error> {
        self.write_bytes(cmd).map_err(|_| Error::Write)?;
        self.write_bytes(b"\r\n").map_err(|_| Error::Write)
    }

    fn read_bytes(&mut self) -> nb::Result<usize, Rx::Error> {
        let mut bytes_read = 0;
        while self.buf.remaining_capacity() > 0 {
            self.buf.push(self.rx.read()?);
            bytes_read += 1;
        }

        Ok(bytes_read)
    }

    fn read_until<'a, C: Condition<'a>>(&'a mut self, condition: C) -> Result<C::Output, Error> {
        if self.read_finished {
            self.read_finished = false;
            self.buf.clear();
        }

        loop {
            match self.read_bytes() {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        return Err(Error::BufferFull);
                    }
                }
                Err(nb::Error::WouldBlock) => {}
                Err(_) => return Err(Error::Read),
            };

            if condition.is_performed(&self.buf) {
                self.read_finished = true;
                break;
            }
        }

        Ok(condition.output(&self.buf))
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
    type Output = Result<&'a [u8], &'a [u8]>;

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
