use arrayvec::ArrayVec;
use embedded_hal::serial;

pub const ADAPTER_BUF_CAPACITY: usize = 512;

const OK: &[u8] = b"OK\r\n";
const READY: &[u8] = b"ready\r\n";

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
        self.read_until(READY)?;

        Ok(())
    }

    pub fn send_at_command(&mut self, raw: &[u8]) -> Result<&'_ [u8], Error> {
        self.send_command(raw)?;
        self.read_until(OK)
    }

    fn disable_echo(&mut self) -> Result<(), Error> {
        self.send_command(b"ATE0")?;
        self.read_until(OK)?;

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

    fn read_until(&mut self, msg: &[u8]) -> Result<&'_ [u8], Error> 
    {
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

            if self.buf.ends_with(msg) {
                self.read_finished = true;
                return Ok(&self.buf[0..self.buf.len() - msg.len()]);
            }
        }
    }
}
