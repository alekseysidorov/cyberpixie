#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie::{
    leds::{SmartLedsWrite, RGB8},
    time::{Microseconds, Milliseconds},
    AppConfig, DeadlineTimer, ImagesRepository,
};
use cyberpixie_firmware::{
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    splash::WanderingLight,
    storage::ImagesStorage,
    TimerImpl,
};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, SoftApConfig};
use gd32vf103xx_hal::{
    pac,
    prelude::*,
    serial::Serial,
    spi::{Spi, MODE_0},
    timer::Timer,
};
use stdio_serial::uprintln;
use ws2812_spi::Ws2812;

const MAX_IMAGE_BUF_SIZE: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = pac::Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut timer = TimerImpl::from(Timer::timer0(dp.TIMER0, 1.mhz(), &mut rcu));

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
    let mut device = {
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
        device
    };
    let images_repository = ImagesStorage::open(&mut device).unwrap();

    uprintln!("Total images count: {}", images_repository.count());

    uprintln!("Showing splash...");
    let splash = WanderingLight::<STRIP_LEDS_COUNT>::default();
    for (ticks, line) in splash {
        timer.set_deadline(Microseconds(ticks));
        strip.write(core::array::IntoIter::new(line)).ok();
        nb::block!(timer.wait_deadline()).unwrap();
    }
    uprintln!("Splash has been showed.");

    let (esp_tx, esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(dp.USART1, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    uprintln!("esp32 serial communication port configured.");

    let ap = {
        let adapter = Adapter::new(esp_rx, esp_tx).unwrap();
        let config = SoftApConfig {
            ssid: "cyberpixie",
            password: "12345678",
            channel: 5,
            mode: 4,
        };
        config.start(adapter).unwrap()
    };
    let network = cyberpixie_firmware::network::into_service(ap);
    uprintln!("SoftAP has been successfuly configured.");

    let mut buf: [RGB8; MAX_IMAGE_BUF_SIZE] = [RGB8::default(); MAX_IMAGE_BUF_SIZE];
    let app = AppConfig::<_, _, _, _, STRIP_LEDS_COUNT> {
        network,
        timer,
        images_repository,
        strip,
    }
    .into_app(&mut buf);

    app.run()
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
