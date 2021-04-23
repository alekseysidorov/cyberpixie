#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::alloc::Layout;

use gd32vf103xx_hal::{delay::McycleDelay, pac, prelude::*, serial::Serial, spi::Spi};
use pixel_poi_firmware::{
    alloc::{heap_bottom, RiscVHeap},
    config::SERIAL_PORT_CONFIG,
    generated, stdout,
    strip::{FixedImage, StripLineSource},
    uprintln,
};
use smart_leds::{SmartLedsWrite, RGB8};
use ws2812_spi::Ws2812;

use panic_halt as _;

#[global_allocator]
static ALLOCATOR: RiscVHeap = RiscVHeap::empty();

unsafe fn init_alloc() {
    // Initialize the allocator BEFORE you use it.
    let start = heap_bottom();
    let size = 1024; // in bytes
    ALLOCATOR.init(start, size)
}

#[riscv_rt::entry]
fn main() -> ! {
    unsafe { init_alloc() }

    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);

    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdout::enable(usb_tx);

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
    let mut delay = McycleDelay::new(&rcu.clocks);
    let mut source = FixedImage::from_raw(&generated::DATA, 300.hz()).unwrap();

    uprintln!("Led strip configured.");
    strip
        .write(core::iter::repeat(RGB8::default()).take(144))
        .ok();
    uprintln!("Led strip cleaned.");

    let vec = alloc::vec![0_u8; 512];
    uprintln!("Successfuly allocated: {}", vec.len());

    loop {
        let (us, line) = source.next_line();
        strip.write(line).ok();
        delay.delay_us(us.0);
    }
}

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    uprintln!("OOM with layout: {:?}", layout);

    loop {
        continue;
    }
}
