#![no_std]
#![no_main]

use panic_halt as _;

use longan_nano_blinky::{
    hal::{delay::McycleDelay, pac, prelude::*},
    led::{rgb, LedControl},
};

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcu = dp.RCU.configure().freeze();

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpioc = dp.GPIOC.split(&mut rcu);

    let (mut red, mut green, mut blue) = rgb(gpioc.pc13, gpioa.pa1, gpioa.pa2);
    let leds: [&mut dyn LedControl; 3] = [&mut red, &mut green, &mut blue];

    let mut delay = McycleDelay::new(&rcu.clocks);

    let mut i = 0;
    let mut time = 12;
    let mut modifier = 1;

    loop {
        let inext = (i + 1) % leds.len();
        leds[i].off();
        leds[inext].on();
        delay.delay_ms(time);

        i = inext;

        time += modifier;
        if time > 250 || time < 10 {
            modifier *= -1;
        }
    }
}
