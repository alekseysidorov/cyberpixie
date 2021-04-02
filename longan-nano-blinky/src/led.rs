//! On-board RGB leds
//!
//! - Red = PC13
//! - Green = PA1
//! - Blue = PA2

use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::gpio::{
    gpioa::{PA1, PA2},
    gpioc::PC13,
    Active, Output, PushPull,
};

/// Red LED
pub struct LedRed {
    port: PC13<Output<PushPull>>,
}

impl LedRed {
    pub fn new<T: Active>(port: PC13<T>) -> Self {
        Self {
            port: port.into_push_pull_output(),
        }
    }
}

/// Green LED
pub struct LedGreen {
    port: PA1<Output<PushPull>>,
}

impl LedGreen {
    pub fn new<T: Active>(port: PA1<T>) -> Self {
        Self {
            port: port.into_push_pull_output(),
        }
    }
}

/// Blue LED
pub struct LedBlue {
    port: PA2<Output<PushPull>>,
}

impl LedBlue {
    pub fn new<T: Active>(port: PA2<T>) -> Self {
        Self {
            port: port.into_push_pull_output(),
        }
    }
}

/// Returns RED, GREEN and BLUE LEDs.
pub fn rgb<X, Y, Z>(red: PC13<X>, green: PA1<Y>, blue: PA2<Z>) -> (LedRed, LedGreen, LedBlue)
where
    X: Active,
    Y: Active,
    Z: Active,
{
    let red: LedRed = LedRed::new(red);
    let green: LedGreen = LedGreen::new(green);
    let blue: LedBlue = LedBlue::new(blue);

    (red, green, blue)
}

/// Generic LED
pub trait LedControl {
    /// Turns the LED off
    fn off(&mut self);

    /// Turns the LED on
    fn on(&mut self);
}

impl LedControl for LedRed {
    fn off(&mut self) {
        self.port.set_high().unwrap();
    }

    fn on(&mut self) {
        self.port.set_low().unwrap();
    }
}

impl LedControl for LedGreen {
    fn off(&mut self) {
        self.port.set_high().unwrap();
    }

    fn on(&mut self) {
        self.port.set_low().unwrap();
    }
}

impl LedControl for LedBlue {
    fn off(&mut self) {
        self.port.set_high().unwrap();
    }

    fn on(&mut self) {
        self.port.set_low().unwrap();
    }
}
