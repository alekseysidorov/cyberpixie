#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use aurora_led_firmware::{config::SERIAL_PORT_CONFIG, stdout, uprint, uprintln};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, Event, SoftAp, SoftApConfig};
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial};

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

#[riscv_rt::entry]
fn main() -> ! {
    // Hardware initialization step.
    let dp = Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut delay = McycleDelay::new(&rcu.clocks);

    let gpioa = dp.GPIOA.split(&mut rcu);

    // Turn on the LED to make it possible to distinguish
    // between normal and boot modes.
    gpioa.pa1.into_push_pull_output().set_high().unwrap();

    let (usb_tx, _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdout::enable(usb_tx);

    delay.delay_ms(1_000);
    uprintln!("Serial port configured.");

    let (esp_tx, esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(dp.USART1, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    uprintln!("esp32 serial communication port configured.");

    let adapter = Adapter::new(esp_rx, esp_tx).unwrap();
    let (mut net_reader, _net_writer) = SoftAp::new(adapter)
        .start(SoftApConfig {
            ssid: "cyberpixie",
            password: "12345678",
            channel: 5,
            mode: 4,
        })
        .unwrap();
    uprintln!("SoftAP has been successfuly configured.");

    loop {
        if let Ok(event) = net_reader.poll_data() {
            match event {
                Event::Closed { link_id } => {
                    uprintln!("Closed {}", link_id);
                }
                Event::DataAvailable { link_id, reader } => {
                    let len = reader.len();
                    for _ in reader {}
                    // uprintln!("Amount of {} bytes have been received from the {}", len, link_id);
                }
                _ => {}
            }
        }
    }
}
