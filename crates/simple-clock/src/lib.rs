#![cfg_attr(not(test), no_std)]

pub trait SimpleClock {
    fn now_us(&self) -> u64;
}

pub struct ElapsedTimer<'a, T> {
    clock: &'a T,
    now: u64,
}

impl<'a, T: SimpleClock> ElapsedTimer<'a, T> {
    pub fn new(clock: &'a T) -> Self {
        Self {
            clock,
            now: clock.now_us(),
        }
    }

    pub fn restart(&mut self) {
        self.now = self.clock.now_us();
    }

    pub fn elapsed(&self) -> u64 {
        self.clock.now_us().saturating_sub(self.now)
    }
}
