use cyberpixie::time::{CountDown, Hertz};
use gd32vf103xx_hal::time as gd32_time;

pub struct TimerImpl<T: CountDown<Time = gd32_time::Hertz>>(T);

impl<T: CountDown<Time = gd32_time::Hertz>> TimerImpl<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    pub fn release(self) -> T {
        self.0
    }
}

impl<T: CountDown<Time = gd32_time::Hertz>> From<T> for TimerImpl<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

impl<T: CountDown<Time = gd32_time::Hertz>> CountDown for TimerImpl<T> {
    type Time = Hertz;

    fn start<C>(&mut self, count: C)
    where
        C: Into<Self::Time>,
    {
        let hz = gd32_time::Hertz(count.into().0);
        if hz.0 > 0 {
            self.0.start(hz)
        }
    }

    fn wait(&mut self) -> nb::Result<(), void::Void> {
        self.0.wait()
    }
}
