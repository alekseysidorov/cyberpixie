//! On-board RGB leds
//!
//! - Red = PC13
//! - Green = PA1
//! - Blue = PA2

use embedded_hal::digital::v2::{InputPin, OutputPin};
use gd32vf103xx_hal::gpio::{
    gpioa::{PA1, PA2},
    gpioc::PC13,
    Floating, Input, Output, PushPull,
};

pub type RedLed = Led<Red>;
pub type GreenLed = Led<Green>;
pub type BlueLed = Led<Blue>;

pub struct Led<T: IntoLed> {
    port: T::Pin,
}

impl<T> Led<T>
where
    T: IntoLed,
    <<T as IntoLed>::Pin as OutputPin>::Error: core::fmt::Debug,
{
    pub fn new(port: T::Port) -> Self {
        Self {
            port: T::wire_pin(port),
        }
    }
}

pub trait LedControl {
    fn off(&mut self);
    fn on(&mut self);
}

impl<T> LedControl for Led<T>
where
    T: IntoLed,
    <<T as IntoLed>::Pin as OutputPin>::Error: core::fmt::Debug,
{
    fn off(&mut self) {
        self.port.set_high().unwrap();
    }

    fn on(&mut self) {
        self.port.set_low().unwrap();
    }
}

pub trait IntoLed {
    type Port: InputPin;
    type Pin: OutputPin;

    fn wire_pin(port: Self::Port) -> Self::Pin;
}

pub struct Red;

impl IntoLed for Red {
    type Port = PC13<Input<Floating>>;
    type Pin = PC13<Output<PushPull>>;

    fn wire_pin(port: Self::Port) -> Self::Pin {
        port.into_push_pull_output()
    }
}

pub struct Green;

impl IntoLed for Green {
    type Port = PA1<Input<Floating>>;
    type Pin = PA1<Output<PushPull>>;

    fn wire_pin(port: Self::Port) -> Self::Pin {
        port.into_push_pull_output()
    }
}

pub struct Blue;

impl IntoLed for Blue {
    type Port = PA2<Input<Floating>>;
    type Pin = PA2<Output<PushPull>>;

    fn wire_pin(port: Self::Port) -> Self::Pin {
        port.into_push_pull_output()
    }
}
