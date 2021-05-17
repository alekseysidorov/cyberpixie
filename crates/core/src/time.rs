pub use cyberpixie_proto::types::Hertz;

macro_rules! impl_time_unit {
    ($name:ident, $hz_factor:expr) => {
        #[derive(PartialEq, PartialOrd, Clone, Copy, Eq, Debug, Ord)]
        pub struct $name(pub u32);

        impl From<u32> for $name {
            fn from(inner: u32) -> Self {
                Self(inner)
            }
        }

        impl From<Hertz> for $name {
            fn from(hz: Hertz) -> Self {
                Self($hz_factor / hz.0)
            }
        }

        impl From<$name> for Hertz {
            fn from(time: $name) -> Self {
                Self($hz_factor / time.0)
            }
        }
    };
}

impl_time_unit!(Microseconds, 1_000_000);
impl_time_unit!(Milliseconds, 1_000);

pub trait DeadlineTimer {
    type Error;

    fn set_deadline<I: Into<Hertz>>(&mut self, timeout: I);

    fn wait_deadline(&mut self) -> nb::Result<(), Self::Error>;

    fn delay<I: Into<Hertz>>(&mut self, timeout: I) -> Result<(), Self::Error> {
        self.set_deadline(timeout);
        nb::block!(self.wait_deadline())
    }
}
