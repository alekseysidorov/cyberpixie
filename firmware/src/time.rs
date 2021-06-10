use cyberpixie::{
    nb_utils::NbResultExt,
    time::{AsyncCountDown, AsyncTimer, CountDown, Hertz},
};
use gd32vf103xx_hal::time as gd32_time;

pub struct TimerWrapper<T: CountDown<Time = gd32_time::Hertz>>(T);

impl<T: CountDown<Time = gd32_time::Hertz>> TimerWrapper<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<T: CountDown<Time = gd32_time::Hertz>> From<T> for TimerWrapper<T> {
    fn from(inner: T) -> Self {
        Self::new(inner)
    }
}

impl<T: CountDown<Time = gd32_time::Hertz>> CountDown for TimerWrapper<T> {
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

impl<T: CountDown<Time = gd32_time::Hertz>> AsyncCountDown for TimerWrapper<T> {
    fn start<C>(&mut self, count: C)
    where
        C: Into<Hertz>,
    {
        let hz = gd32_time::Hertz(count.into().0);
        if hz.0 > 0 {
            self.0.start(hz)
        }
    }

    fn poll_wait(&mut self, ctx: &mut core::task::Context<'_>) -> core::task::Poll<()> {
        self.0.wait().into_poll(ctx).map(drop)
    }
}

pub fn new_async_timer<T: CountDown<Time = gd32_time::Hertz>>(
    inner: T,
) -> AsyncTimer<impl AsyncCountDown> {
    AsyncTimer::new(TimerWrapper::new(inner))
}
