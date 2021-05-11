use cyberpixie_proto::types::Hertz;

/// Time unit
#[derive(PartialEq, PartialOrd, Clone, Copy, Eq, Debug)]
pub struct Microseconds(pub u32);

impl Microseconds {
    pub fn to_ms(self) -> u32 {
        self.0 / 1_000
    }
}

impl From<u32> for Microseconds {
    fn from(inner: u32) -> Self {
        Self(inner)
    }
}

impl From<Hertz> for Microseconds {
    fn from(hz: Hertz) -> Self {
        Self(1_000_000 / hz.0)
    }
}
