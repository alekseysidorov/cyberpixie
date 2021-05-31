pub use cyberpixie_proto::Hertz;
pub use embedded_hal::timer::CountDown;

use core::{future::Future, time::Duration};

use cyberpixie_proto::until_ok;

macro_rules! impl_time_unit {
    ($name:ident, $hz_factor:expr) => {
        #[derive(PartialEq, PartialOrd, Clone, Copy, Eq, Debug, Ord)]
        pub struct $name(pub u32);

        impl $name {
            pub const SECS_FACTOR: u32 = $hz_factor;
        }

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

impl From<Milliseconds> for Microseconds {
    fn from(ms: Milliseconds) -> Self {
        Self(ms.0 * 1_000)
    }
}

impl_time_unit!(Microseconds, 1_000_000);
impl_time_unit!(Milliseconds, 1_000);

pub trait CountDownEx: CountDown {
    type WaitFuture<'a>: Future<Output = ()>;

    fn delay_us<I: Into<Microseconds>>(&mut self, timeout: I);

    fn delay_ms<I: Into<Milliseconds>>(&mut self, timeout: I);

    fn delay(&mut self, duration: Duration) {
        let secs = duration.as_secs();
        if secs > 0 {
            self.delay_ms(Milliseconds(secs as u32 * Milliseconds::SECS_FACTOR));
        }

        let us = duration.subsec_micros();
        if us > 0 {
            self.delay_us(Microseconds(us));
        }
    }

    fn wait_async(&mut self, duration: Duration) -> Self::WaitFuture<'_>;
}

impl<T> CountDownEx for T
where
    T: CountDown<Time = Hertz> + 'static,
{
    type WaitFuture<'a> = impl Future<Output = ()> + 'a;

    fn delay_us<I: Into<Microseconds>>(&mut self, timeout: I) {
        let mut timeout = timeout.into();
        while timeout.0 > Microseconds::SECS_FACTOR {
            self.start(Microseconds::SECS_FACTOR);
            nb::block!(self.wait()).ok();
            timeout.0 -= Microseconds::SECS_FACTOR;
        }

        self.start(timeout);
        nb::block!(self.wait()).ok();
    }

    fn delay_ms<I: Into<Milliseconds>>(&mut self, timeout: I) {
        let mut timeout = timeout.into();
        while timeout.0 > Milliseconds::SECS_FACTOR {
            self.start(Milliseconds::SECS_FACTOR);
            nb::block!(self.wait()).ok();
            timeout.0 -= Milliseconds::SECS_FACTOR;
        }

        self.start(timeout);
        nb::block!(self.wait()).ok();
    }

    fn wait_async(&mut self, duration: Duration) -> Self::WaitFuture<'_> {
        let mut secs = duration.as_secs() as u32;
        let us = duration.subsec_micros();

        async move {
            while secs > 0 {
                self.start(Hertz(1));
                until_ok(|| self.wait()).await.ok();
                secs -= 1;
            }

            if us > 0 {
                self.start(Microseconds(us));
                until_ok(|| self.wait()).await.ok();
            }
        }
    }
}
