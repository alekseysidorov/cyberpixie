pub use cyberpixie_proto::types::Hertz;
pub use embedded_hal::timer::CountDown;

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

pub trait CountDownEx: CountDown {
    fn delay<I: Into<Self::Time>>(&mut self, timeout: I);
}

impl<T: CountDown> CountDownEx for T {
    fn delay<I: Into<Self::Time>>(&mut self, timeout: I) {
        self.start(timeout);
        nb::block!(self.wait()).ok();
    }
}
