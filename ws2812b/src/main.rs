#![no_std]
#![no_main]

use longan_nano::{
    hal::{
        delay::McycleDelay,
        pac::{self},
        prelude::*,
        time::MegaHertz,
        timer::Timer,
    },
    sprintln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_timer_delay::Ws2812;

use panic_halt as _;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpiob = dp.GPIOB.split(&mut rcu);

    longan_nano::stdout::configure(
        dp.USART0,
        gpioa.pa9,
        gpioa.pa10,
        115_200.bps(),
        &mut afio,
        &mut rcu,
    );

    let timer = Timer::timer0(dp.TIMER0, MegaHertz(3), &mut rcu);
    let mut delay = McycleDelay::new(&rcu.clocks);

    let mut strip = Ws2812::new(timer, gpiob.pb5.into_push_pull_output());

    sprintln!("ws2812b configured, sending a couple of colors...");

    let mut data = [RGB8::default(); 4];
    let empty = [RGB8::default(); 4];

    data[0] = RGB8 {
        r: 0,
        g: 0,
        b: 10,
    };
    data[1] = RGB8 {
        r: 0,
        g: 10,
        b: 0,
    };
    data[2] = RGB8 {
        r: 10,
        g: 0,
        b: 0,
    };
    data[3] = RGB8 {
        r: 5,
        g: 7,
        b: 8,
    };

    delay.delay_ms(1000);
    strip.write(empty.iter().cloned()).unwrap();

    loop {
        delay.delay_ms(1000);
        strip.write(data.iter().cloned()).unwrap();

        delay.delay_ms(1000);
        strip.write(empty.iter().cloned()).unwrap();

        sprintln!("Blink cycle finished");
    }
}
