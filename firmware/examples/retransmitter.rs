#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use aurora_led_firmware::{config::SERIAL_PORT_CONFIG, stdout, uprintln};
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

    // Hardware initialization step.

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
    stdout::enable(usb_tx);

    delay.delay_ms(1_000);
    uprintln!("Serial port configured.");

    let (mut esp_tx, mut esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(dp.USART1, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    uprintln!("esp32 serial communication port configured.");

    loop {
        match (usb_rx.read(), esp_rx.read()) {
            (Ok(u), Ok(w)) => {
                esp_tx.write(u).ok();
                stdout::write_byte(w).ok();
                continue;
            }
            (Ok(u), Err(nb::Error::WouldBlock)) => {
                esp_tx.write(u).ok();
                continue;
            }
            (Err(nb::Error::WouldBlock), Ok(w)) => {
                stdout::write_byte(w).ok();
                continue;
            }
            (Err(nb::Error::WouldBlock), Err(nb::Error::WouldBlock)) => continue,
            _ => {}
        };
    }
}
