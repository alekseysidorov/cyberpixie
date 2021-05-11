/// Time unit
#[derive(PartialEq, PartialOrd, Clone, Copy, Eq, Debug)]
pub struct Microseconds(pub u32);

impl Microseconds {
    pub fn to_ms(self) -> u32 {
        self.0 / 1_000
    }

    pub fn from_hertz(hz: u32) -> Self {
        Self(1_000_000 / hz)
    }
}

impl From<u32> for Microseconds {
    fn from(inner: u32) -> Self {
        Self(inner)
    }
}
