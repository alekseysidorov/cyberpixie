use cyberpixie::{time::Hertz, DeadlineTimer};
use embedded_hal::timer::CountDown;
use gd32vf103xx_hal::time as gd32_time;
use void::Void;

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

impl<T: CountDown<Time = gd32_time::Hertz>> DeadlineTimer for TimerImpl<T> {
    type Error = Void;

    fn deadline<I: Into<Hertz>>(&mut self, timeout: I) {
        let hz = timeout.into();
        let count = gd32_time::Hertz(hz.0);

        if hz.0 > 0 {
            self.0.start(count)
        }
    }

    fn wait_deadline(&mut self) -> nb::Result<(), Self::Error> {
        self.0.wait()
    }
}
