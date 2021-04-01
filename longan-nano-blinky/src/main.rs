#![no_std]
#![no_main]

use panic_halt as _;

use longan_nano_blinky::{
    hal::{delay::McycleDelay, pac, prelude::*},
    led::{BlueLed, GreenLed, LedControl, RedLed},
};
use riscv_rt::entry;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcu = dp.RCU.configure().freeze();

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpioc = dp.GPIOC.split(&mut rcu);

    let mut red = RedLed::new(gpioc.pc13);
    let mut green = GreenLed::new(gpioa.pa1);
    let mut blue = BlueLed::new(gpioa.pa2);

    let leds: [&mut dyn LedControl; 3] = [&mut red, &mut green, &mut blue];

    let mut delay = McycleDelay::new(&rcu.clocks);

    let mut i = 0;

    loop {
        let inext = (i + 1) % leds.len();
        leds[i].off();
        leds[inext].on();
        delay.delay_ms(50);

        i = inext;
    }
}
