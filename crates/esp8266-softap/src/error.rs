#[derive(Debug)]
pub enum Error<R, W> {
    Read(R),
    Write(W),
    Format,
    BufferFull,
    MalformedCommand {
        cmd: &'static str,
        msg: &'static str,
    },
    Timeout,
}

pub type Result<T, R, W> = core::result::Result<T, Error<R, W>>;
