#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie_firmware::{
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG},
    splash::WanderingLight,
    storage::ImagesRepository,
    strip::{FixedImage, StripLineSource},
};
use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::{
    delay::McycleDelay,
    pac,
    prelude::*,
    serial::Serial,
    spi::{Spi, MODE_0},
};
use smart_leds::{SmartLedsWrite, RGB8};
use stdio_serial::uprintln;
use ws2812_spi::Ws2812;

const MAX_STRIP_LEN: usize = 144;
const TICK_DELAY: u32 = (MAX_STRIP_LEN / STRIP_LEN) as u32;

const STRIP_LEN: usize = 144;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut delay = McycleDelay::new(&rcu.clocks);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    delay.delay_ms(1_000);
    uprintln!("Serial port configured.");

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
    let mut strip = Ws2812::new(spi);

    uprintln!("Led strip configured.");
    strip
        .write(core::iter::repeat(RGB8::default()).take(144))
        .ok();
    uprintln!("Led strip cleaned.");

    let mut splash = WanderingLight::<STRIP_LEN>::default();

    for (ticks, line) in splash.cycle() {
        strip.write(core::array::IntoIter::new(line)).ok();
        delay.delay_us(TICK_DELAY * ticks);
    }

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    uprintln!();
    uprintln!("The firmware panicked!");
    uprintln!("- {}", info);

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
