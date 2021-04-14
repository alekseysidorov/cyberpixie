#![no_std]
#![no_main]

use longan_nano::{
    hal::{
        delay::McycleDelay,
        pac::{self},
        prelude::*,
        spi::Spi,
    },
    sprintln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

use panic_halt as _;

struct TickerLine<C: Iterator<Item = RGB8>, const N: usize> {
    colors: C,
    current_led: usize,
}

impl<C: Iterator<Item = RGB8>, const N: usize> TickerLine<C, N> {
    fn new(colors: C) -> Self {
        Self {
            colors,
            current_led: N - 1,
        }
    }

    pub fn clear_line(&self) -> impl Iterator<Item = RGB8> {
        core::iter::repeat(RGB8::default()).take(N)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn next_line<'a>(&'a mut self) -> impl Iterator<Item = RGB8> + 'a {
        self.current_led = (self.current_led + 1) % N;
        (0..N).map(move |active_led| {
            if active_led == self.current_led {
                self.colors.next().unwrap()
            } else {
                RGB8::default()
            }
        })
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    longan_nano::stdout::configure(
        dp.USART0,
        gpioa.pa9,
        gpioa.pa10,
        115_200.bps(),
        &mut afio,
        &mut rcu,
    );

    let mut delay = McycleDelay::new(&rcu.clocks);

    let spi = {
        let pins = (
            gpioa.pa5.into_alternate_push_pull(),
            gpioa.pa6.into_floating_input(),
            gpioa.pa7.into_alternate_push_pull(),
        );

        Spi::spi0(
            dp.SPI0,
            pins,
            &mut afio,
            ws2812_spi::MODE,
            2800.khz(),
            &mut rcu,
        )
    };

    const LED_COUNT: usize = 25;
    const LED_BRIGHTNESS: u8 = 5;

    let mut strip = Ws2812::new(spi);
    strip
        .write(core::iter::repeat(RGB8::default()).take(144))
        .ok();

    sprintln!("ws2812b configured, sending a couple of colors...");

    let colors = [
        RGB8 {
            r: LED_BRIGHTNESS,
            g: 0,
            b: 0,
        },
        RGB8 {
            r: 0,
            g: LED_BRIGHTNESS,
            b: 0,
        },
        RGB8 {
            r: 0,
            g: 0,
            b: LED_BRIGHTNESS,
        },
    ]
    .iter()
    .copied()
    .cycle();

    let mut lines = TickerLine::<_, LED_COUNT>::new(colors);

    strip.write(lines.clear_line()).unwrap();

    loop {
        delay.delay_us(20);
        // delay.delay_ms(100);
        strip.write(lines.next_line()).unwrap();
    }
}
