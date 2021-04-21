#![no_std]
#![no_main]

use gd32vf103xx_hal::{delay::McycleDelay, pac, prelude::*, serial::Serial, spi::Spi};
use pixel_poi_firmware::{
    config::SERIAL_PORT_CONFIG,
    generated,
    strip::{FixedImage, StripLineSource},
    uwriteln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

use panic_halt as _;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);

    let (mut usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };

    uwriteln!(usb_tx, "Serial port configured.");

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
    let mut delay = McycleDelay::new(&rcu.clocks);
    let mut source = FixedImage::from_raw(&generated::DATA, 2.hz()).unwrap();

    uwriteln!(usb_tx, "Led strip configured.");
    strip
        .write(core::iter::repeat(RGB8::default()).take(144))
        .ok();
    uwriteln!(usb_tx, "Led strip cleaned.");

    let rate = 1_000.hz();
    loop {
        let (us, line) = source.next_line();
        strip.write(line).ok();
        delay.delay_us(1_000_000 / rate.0);
    }
}
