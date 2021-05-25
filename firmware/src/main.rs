#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
    time::Duration,
};

use cyberpixie::{
    leds::SmartLedsWrite,
    stdio::uprintln,
    time::{CountDown, CountDownEx, Microseconds},
    AppConfig, ImagesRepository,
};
use cyberpixie_firmware::{
    config::{SERIAL_PORT_CONFIG, SOFTAP_CONFIG, STRIP_LEDS_COUNT},
    irq::{self},
    splash::WanderingLight,
    storage::ImagesStorage,
    NextImageBtn, TimerImpl,
};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, ADAPTER_BUF_CAPACITY};
use gd32vf103xx_hal::{
    pac::{self},
    prelude::*,
    serial::{Event as SerialEvent, Serial},
    spi::{Spi, MODE_0},
    timer::Timer,
};
use ws2812_spi::Ws2812;

#[export_name = "TIMER1"]
unsafe fn handle_uart1_interrupt() {
    irq::handle_usart1_update()
}

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

        let mut serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.listen(SerialEvent::Rxne);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    timer.delay(Duration::from_secs(2));
    uprintln!();
    uprintln!("Welcome to Cyberpixie serial console!");

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
    uprintln!("Ws2812 strip configured.");

    let device = {
        let gpiob = dp.GPIOB.split(&mut rcu);
        let spi = Spi::spi1(
            dp.SPI1,
            (
                gpiob.pb13.into_alternate_push_pull(),
                gpiob.pb14.into_floating_input(),
                gpiob.pb15.into_alternate_push_pull(),
            ),
            MODE_0,
            20.mhz(),
            &mut rcu,
        );

        let mut cs = gpiob.pb12.into_push_pull_output();
        cs.set_low().unwrap();

        let mut device = embedded_sdmmc::SdMmcSpi::new(spi, cs);
        device.init().unwrap();
        device
    };
    let images = ImagesStorage::open(device).unwrap();
    uprintln!("Total images count: {}", images.count());

    uprintln!("Showing splash...");
    let splash = WanderingLight::<STRIP_LEDS_COUNT>::default();
    for (ticks, line) in splash {
        timer.start(Microseconds(ticks));
        strip.write(core::array::IntoIter::new(line)).ok();
        nb::block!(timer.wait()).ok();
    }
    uprintln!("Splash has been showed.");

    uprintln!("Enabling esp32 serial device");
    let mut esp_en = gpioa.pa4.into_push_pull_output();
    esp_en.set_high().ok();
    timer.delay(Duration::from_secs(3));
    uprintln!("esp32 device has been enabled");

    let (esp_tx, esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(dp.USART1, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };

    let esp_rx = irq::init_interrupts(irq::Usart1 {
        rx: esp_rx,
        timer: Timer::timer1(dp.TIMER1, 15.khz(), &mut rcu),
    });
    uprintln!("esp32 serial communication port configured.");
    let ap = SOFTAP_CONFIG
        .start(Adapter::new(esp_rx, esp_tx).unwrap())
        .unwrap();
    let network = cyberpixie_firmware::transport::TransportImpl::new(ap);
    uprintln!("SoftAP has been successfuly configured.");

    let mut events = NextImageBtn::new(gpioa.pa8.into_pull_down_input());

    AppConfig::<_, _, _, _, STRIP_LEDS_COUNT, ADAPTER_BUF_CAPACITY> {
        network,
        timer,
        images: &images,
        strip,
        device_id: cyberpixie_firmware::device_id(),
        events: &mut events,
    }
    .into_event_loop()
    .run()
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
