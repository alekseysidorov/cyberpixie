#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie_firmware::{
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    images::ImagesRepository,
    storage::ImagesStorage,
    time::{DeadlineTimer, Milliseconds},
};
use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::{
    pac,
    prelude::*,
    serial::Serial,
    spi::{Spi, MODE_0},
    timer::Timer,
};
use heapless::Vec;
use smart_leds::{SmartLedsWrite, RGB8};
use stdio_serial::uprintln;
use ws2812_spi::Ws2812;

const MAX_IMAGE_BUF_SIZE: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut timer = Timer::timer0(dp.TIMER0, 1.mhz(), &mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    timer.delay(Milliseconds(1_000)).unwrap();
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

    // SPI1_SCK(PB13), SPI1_MISO(PB14) and SPI1_MOSI(PB15) GPIO pin configuration
    let gpiob = dp.GPIOB.split(&mut rcu);
    let spi = Spi::spi1(
        dp.SPI1,
        (
            gpiob.pb13.into_alternate_push_pull(),
            gpiob.pb14.into_floating_input(),
            gpiob.pb15.into_alternate_push_pull(),
        ),
        MODE_0,
        20.mhz(), // 16.mzh()
        &mut rcu,
    );

    let mut cs = gpiob.pb12.into_push_pull_output();
    cs.set_low().unwrap();

    let mut device = embedded_sdmmc::SdMmcSpi::new(spi, cs);
    device.init().unwrap();

    let mut images = ImagesStorage::open(&mut device).unwrap();
    uprintln!("Total images count: {}", images.count());

    let mut buf: Vec<RGB8, MAX_IMAGE_BUF_SIZE> = Vec::new();

    let image_num = 1;
    let (refresh_rate, data) = images.read_image(image_num);
    buf.extend(data);

    uprintln!("Loaded {} image from the repository", image_num);

    let mut lines = buf.chunks_exact(STRIP_LEDS_COUNT).cycle();
    loop {
        timer.deadline(refresh_rate);

        let line = lines.next().unwrap();
        strip.write(line.iter().copied()).unwrap();

        nb::block!(timer.wait_deadline()).unwrap();
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
