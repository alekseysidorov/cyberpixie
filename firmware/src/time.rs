use cyberpixie::{
    nb_utils::NbResultExt,
    time::{AsyncCountDown, AsyncTimer, CountDown, Hertz},
};
use esp8266_softap::clock::SimpleClock;
use gd32vf103xx_hal::{rcu::Clocks, time as gd32_time};

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

/// Machine mode cycle counter (`mcycle`) as a simple clock provider
#[derive(Copy, Clone, Debug)]
pub struct McycleClock {
    core_frequency: u64,
}

impl McycleClock {
    /// Constructs the simple clock provider.
    pub fn new(clocks: &Clocks) -> Self {
        Self {
            core_frequency: clocks.sysclk().0 as u64,
        }
    }
}

impl SimpleClock for McycleClock {
    fn now_us(&self) -> u64 {
        let to = riscv::register::mcycle::read64();
        (to / self.core_frequency) * 1_000_000
    }
}
