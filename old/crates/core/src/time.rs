use core::{
    task::{Context, Poll},
    time::Duration,
};

pub use cyberpixie_proto::Hertz;
pub use embedded_hal::timer::CountDown;

use crate::futures::future;

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

pub trait AsyncCountDown {
    /// Starts a new count down
    fn start<T>(&mut self, count: T)
    where
        T: Into<Hertz>;

    fn poll_wait(&mut self, cx: &mut Context<'_>) -> Poll<()>;
}

pub struct AsyncTimer<T: AsyncCountDown>(T);

impl<T: AsyncCountDown> AsyncTimer<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn start<H>(&mut self, count: H)
    where
        H: Into<Hertz>,
    {
        self.0.start(count)
    }

    pub async fn wait(&mut self) {
        future::poll_fn(|ctx| self.0.poll_wait(ctx)).await
    }

    pub async fn delay(&mut self, duration: Duration) {
        let mut secs = duration.as_secs() as u32;
        let us = duration.subsec_micros();

        while secs > 0 {
            self.start(Hertz(1));
            self.wait().await;
            secs -= 1;
        }

        if us > 0 {
            self.start(Microseconds(us));
            self.wait().await;
        }
    }
}
