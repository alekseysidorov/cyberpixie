#![no_std]
#![no_main]

use core::{fmt::Write, iter::repeat, panic::PanicInfo, sync::atomic, time::Duration};

use atomic::Ordering;
use cyberpixie::{
    leds::SmartLedsWrite, proto::DeviceRole, stdio::uprintln, time::Microseconds, App, Storage,
};
use cyberpixie_firmware::{
    config::{SERIAL_PORT_CONFIG, SOFTAP_CONFIG, STRIP_LEDS_COUNT},
    irq, new_async_timer,
    splash::WanderingLight,
    transport, NextImageBtn, StorageImpl, BLUE_LED, RED_LED,
};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, SoftApConfig};
use gd32vf103xx_hal::{
    pac::{self},
    prelude::*,
    serial::{Event as SerialEvent, Serial},
    spi::{Spi, MODE_0},
    timer::Timer,
};
use heapless::String;
use smart_leds::RGB8;
use transport::TransportImpl;
use ws2812_spi::Ws2812;

#[export_name = "TIMER1"]
unsafe fn handle_uart1_interrupt() {
    irq::handle_usart1_update()
}

async fn run_main_loop(dp: pac::Peripherals) -> ! {
    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut timer = new_async_timer(Timer::timer0(dp.TIMER0, 1.mhz(), &mut rcu));

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let mut serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.listen(SerialEvent::Rxne);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    timer.delay(Duration::from_secs(2)).await;
    uprintln!();
    uprintln!("Welcome to slave example serial console!");

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
    strip
        .write(repeat(RGB8::default()).take(STRIP_LEDS_COUNT))
        .ok();
    uprintln!("Ws2812 strip configured.");

    strip.write(RED_LED.iter().copied()).ok();
    uprintln!("Enabling esp32 serial device");
    let mut esp_en = gpioa.pa4.into_push_pull_output();
    esp_en.set_high().ok();
    timer.delay(Duration::from_secs(5)).await;
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

    let mut adapter = Adapter::new(esp_rx, esp_tx).unwrap();
    uprintln!("{:?}", adapter.list_wifi_ap().unwrap());

    // let device_id = cyberpixie_firmware::device_id();
    // let mut ssid: String<64> = String::new();
    // ssid.write_fmt(core::format_args!(
    //     "cyberpixie_{:X}{:X}{:X}",
    //     device_id[1],
    //     device_id[2],
    //     device_id[3]
    // ))
    // .unwrap();

    // let softap_config = SoftApConfig {
    //     ssid: &ssid,
    //     ..SOFTAP_CONFIG
    // };

    // let mut ap = softap_config
    //     .start()
    //     .unwrap();
    // uprintln!("SoftAP has been successfuly configured with ssid {}.", ssid);
    // strip.write(BLUE_LED.iter().copied()).ok();

    // uprintln!("{:?}", ap.list_wifi_ap().unwrap());

    // let network = TransportImpl::new(ap);

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    direct_executor::run_spinning(run_main_loop(dp))
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // uprintln!();
    // uprintln!("The firmware panicked!");
    // uprintln!("- {}", info);

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
