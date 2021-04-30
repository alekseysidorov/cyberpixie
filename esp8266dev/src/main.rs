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
    let (usb_tx, mut _usb_rx) = {
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
        continue;
    }
}

// use longan_nano::hal::{
//     pac::{self, USART0},
//     prelude::*,
//     serial::{Config, Parity, Serial, StopBits, Tx},
//     timer::Timer,
// };

// use panic_halt as _;

// struct SerialWrapper<'a>(&'a mut Tx<USART0>);

// impl<'a> core::fmt::Write for SerialWrapper<'a> {
//     fn write_str(&mut self, s: &str) -> core::fmt::Result {
//         for byte in s.as_bytes() {
//             if *byte == '\n' as u8 {
//                 let res = nb::block!(self.0.write('\r' as u8));

//                 if res.is_err() {
//                     return Err(::core::fmt::Error);
//                 }
//             }

//             let res = nb::block!(self.0.write(*byte));

//             if res.is_err() {
//                 return Err(::core::fmt::Error);
//             }
//         }
//         Ok(())
//     }
// }

// fn write_str(tx: &mut Tx<USART0>, s: &str) {
//     use core::fmt::Write;
//     SerialWrapper(tx).write_str(s).unwrap()
// }

// fn write_fmt(tx: &mut Tx<USART0>, args: core::fmt::Arguments) {
//     use core::fmt::Write;
//     SerialWrapper(tx).write_fmt(args).unwrap()
// }

// /// Macro for printing to the serial standard output, with a newline.
// #[macro_export]
// macro_rules! sprintln {
//     ($o:expr) => {
//         $crate::write_str($o, "\n")
//     };
//     ($o:expr, $s:expr) => {
//         $crate::write_str($o, concat!($s, "\n"))
//     };
//     ($o:expr, $s:expr, $($tt:tt)*) => {
//         $crate::write_fmt($o, format_args!(concat!($s, "\n"), $($tt)*))
//     };
// }

// #[riscv_rt::entry]
// fn main() -> ! {
//     // Hardware initialization step.
//     let dp = pac::Peripherals::take().unwrap();

//     let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
//     let mut afio = dp.AFIO.constrain(&mut rcu);

//     let gpioa = dp.GPIOA.split(&mut rcu);

//     let (mut usb_tx, mut usb_rx) = {
//         let tx = gpioa.pa9.into_alternate_push_pull();
//         let rx = gpioa.pa10.into_floating_input();
//         let config = Config {
//             baudrate: 115200.bps(),
//             parity: Parity::ParityNone,
//             stopbits: StopBits::STOP1,
//         };
//         let serial = Serial::new(dp.USART0, (tx, rx), config, &mut afio, &mut rcu);
//         serial.split()
//     };

//     let (mut wifi_tx, mut wifi_rx) = {
//         let timer = Timer::timer0(dp.TIMER0, 1.khz(), &mut rcu);

//         let tx = gpioa.pa2.into_alternate_push_pull();
//         let rx = gpioa.pa3.into_floating_input();
//         let config = Config {
//             baudrate: 115200.bps(),
//             parity: Parity::ParityNone,
//             stopbits: StopBits::STOP1,
//         };
//         let serial = Serial::new(dp.USART1, (tx, rx), config, &mut afio, &mut rcu);
//         serial.split()
//     };

//     sprintln!(&mut usb_tx, "Establishing ESP communication");

//     loop {
//         let (ue, we) = match (usb_rx.read(), wifi_rx.read()) {
//             (Ok(u), Ok(w)) => {
//                 wifi_tx.write(u).ok();
//                 usb_tx.write(w).ok();
//                 continue;
//             }
//             (Ok(u), Err(nb::Error::WouldBlock)) => {
//                 wifi_tx.write(u).ok();
//                 continue;
//             }
//             (Err(nb::Error::WouldBlock), Ok(w)) => {
//                 usb_tx.write(w).ok();
//                 continue;
//             }
//             (Err(nb::Error::WouldBlock), Err(nb::Error::WouldBlock)) => continue,
//             (ue, we) => (ue, we),
//         };

//         sprintln!(&mut usb_tx, "Something went wrong: {:?}. {:?}", ue, we);
//     }
// }
