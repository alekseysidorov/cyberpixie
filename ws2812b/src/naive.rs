use embedded_hal::digital::v2::OutputPin;
use longan_nano::hal::rcu::Clocks;

#[derive(Clone, Copy)]
pub struct DelayNs {
    core_frequency: u64,
}

impl DelayNs {
    pub fn new(clocks: &Clocks) -> Self {
        Self {
            core_frequency: clocks.sysclk().0 as u64,
        }
    }

    pub fn delay_ns(&self, ns: u64) {
        let t0 = riscv::register::mcycle::read64();
        let clocks = (ns * self.core_frequency) / 1_000;
        while riscv::register::mcycle::read64().wrapping_sub(t0) <= clocks {}
    }

    pub fn delay_us(&self, us: u64) {
        let t0 = riscv::register::mcycle::read64();
        let clocks = (us * self.core_frequency) / 1_000_000;
        while riscv::register::mcycle::read64().wrapping_sub(t0) <= clocks {}
    }
}

pub struct NaiveDriver<P>
where
    P: OutputPin,
{
    pin: P,
    delay: DelayNs,
}

impl<P> NaiveDriver<P>
where
    P: OutputPin,
{
    pub fn new(pin: P, clocks: &Clocks) -> Self {
        Self {
            pin,
            delay: DelayNs::new(clocks),
        }
    }

    pub fn enable(&mut self) -> Result<(), P::Error> {
        self.pin.set_high()
    }

    pub fn send_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<(), P::Error> {
        self.send_pixel(g)?;
        self.send_pixel(r)?;
        self.send_pixel(b)
    }

    pub fn send_pixel(&mut self, pixel: u8) -> Result<(), P::Error> {
        let mask = 0b10000000_u8;
        for i in 0..8 {
            if (mask >> i) & pixel == 0 {
                self.send_zero()?;
            } else {
                self.send_one()?;
            }
        }

        Ok(())
    }

    pub fn send_reset(&mut self) -> Result<(), P::Error> {
        self.pin.set_low()?;
        self.delay.delay_us(50);
        self.pin.set_high()?;

        Ok(())
    }

    fn send_zero(&mut self) -> Result<(), P::Error> {
        self.delay.delay_ns(400);
        self.pin.set_low()?;
        self.delay.delay_ns(850);
        self.pin.set_high()
    }

    fn send_one(&mut self) -> Result<(), P::Error> {
        self.delay.delay_ns(800);
        self.pin.set_low()?;
        self.delay.delay_ns(450);
        self.pin.set_high()
    }
}
