use gd32vf103xx_hal::time::{Hertz, MilliSeconds};

/// Time unit
#[derive(PartialEq, PartialOrd, Clone, Copy, Eq, Debug)]
pub struct MicroSeconds(pub u32);

impl MicroSeconds {
    pub fn to_ms(self) -> u32 {
        self.0 / 1_000
    }
}

impl From<MilliSeconds> for MicroSeconds {
    fn from(ms: MilliSeconds) -> Self {
        Self(ms.0 * 1000)
    }
}

impl From<u32> for MicroSeconds {
    fn from(inner: u32) -> Self {
        Self(inner)
    }
}

impl From<Hertz> for MicroSeconds {
    fn from(hz: Hertz) -> Self {
        Self(1_000_000 / hz.0)
    }
}
