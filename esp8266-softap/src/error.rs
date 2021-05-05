#[derive(Debug)]
pub enum Error {
    Read,
    Write,
    BufferFull,
    MalformedCommand { cmd: &'static str, msg: &'static str, },
}

pub type Result<T> = core::result::Result<T, Error>;
