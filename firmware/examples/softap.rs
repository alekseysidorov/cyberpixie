#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie::proto::transport::{PacketData, Transport};
use cyberpixie_firmware::{config::SERIAL_PORT_CONFIG, transport::TransportImpl};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, SoftApConfig};
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial};
use stdio_serial::{uprint, uprintln};

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

    let (usb_tx, mut usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    stdio_serial::init(usb_tx);

    delay.delay_ms(1_000);
    uprintln!("Serial port configured.");

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
    let mut transport = TransportImpl::new(ap);
    uprintln!("SoftAP has been successfuly configured.");

    loop {
        let packet = match transport.poll_next_packet() {
            Ok(packet) => packet,
            Err(nb::Error::WouldBlock) => continue,
            Err(nb::Error::Other(err)) => panic!("transport: {:?}", err),
        };

        match packet.data {
            PacketData::Payload(payload) => {
                for byte in payload {
                    uprint!("{}", byte as char);
                }
                transport.request_next_packet(packet.address).unwrap();
            }
            PacketData::RequestNext => unreachable!(),
        }

        let byte = match usb_rx.read() {
            Ok(byte) => byte,
            Err(nb::Error::WouldBlock) => continue,
            Err(nb::Error::Other(err)) => panic!("uart: {:?}", err),
        };

        let to = 0;
        let bytes = [byte];
        transport.send_packet(&bytes, to).unwrap();
        nb::block!(transport.wait_for_next_request(to)).unwrap();
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
