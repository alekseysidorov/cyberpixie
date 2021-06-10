#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie::stdio::uprintln;
use cyberpixie_firmware::config::{ESP32_SERIAL_PORT_CONFIG, SERIAL_PORT_CONFIG};
use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial};

#[riscv_rt::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let mut delay = McycleDelay::new(&rcu.clocks);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    delay.delay_ms(2_000);
    uprintln!("Serial port configured.");

    uprintln!("Enabling esp32 serial device");
    let mut esp_en = gpioa.pa4.into_push_pull_output();
    esp_en.set_low().unwrap();
    delay.delay_ms(2_000);

    esp_en.set_high().unwrap();
    delay.delay_ms(2_000);
    uprintln!("esp32 device has been enabled");

    let (mut esp_tx, mut esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(dp.USART1, (tx, rx), ESP32_SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    uprintln!("esp32 serial communication port configured.");

    loop {
        let byte = nb::block!(usb_rx.read());
        uprintln!("{:?}", byte);
    }

    loop {
        match (usb_rx.read(), esp_rx.read()) {
            (Ok(u), Ok(w)) => {
                esp_tx.write(u).ok();
                stdio_serial::write_byte(w).ok();
                continue;
            }
            (Ok(u), Err(nb::Error::WouldBlock)) => {
                esp_tx.write(u).ok();
                continue;
            }
            (Err(nb::Error::WouldBlock), Ok(w)) => {
                stdio_serial::write_byte(w).ok();
                continue;
            }
            (Err(nb::Error::WouldBlock), Err(nb::Error::WouldBlock)) => continue,
            _ => {}
        };
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
