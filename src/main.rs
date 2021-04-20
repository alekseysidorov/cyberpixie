#![no_std]
#![no_main]

use gd32vf103xx_hal::{delay::McycleDelay, pac, prelude::*, serial::Serial};
use pixel_poi_firmware::{
    config::SERIAL_PORT_CONFIG,
    generated,
    strip::{FixedImage, StripLineSource},
    uwrite, uwriteln,
};

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
    let mut delay = McycleDelay::new(&rcu.clocks);

    let mut source = FixedImage::from_raw(&generated::DATA, 50.ms()).unwrap();
    loop {
        let (us, line) = source.next_line();
        for pixel in line {
            uwrite!(usb_tx, "{}|{}|{} ", pixel.r, pixel.g, pixel.b);
        }
        uwriteln!(usb_tx);

        delay.delay_us(us.0);
        continue;
    }
}
