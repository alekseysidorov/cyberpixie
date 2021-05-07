#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::{
    alloc::Layout,
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie_firmware::{
    allocator::{heap_bottom, RiscVHeap},
    config::SERIAL_PORT_CONFIG,
};
use cyberpixie_proto::{IncomingMessage, PacketReader};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{Adapter, Event, SoftAp, SoftApConfig};
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial};
use stdio_serial::uprintln;

#[global_allocator]
static ALLOCATOR: RiscVHeap = RiscVHeap::empty();

unsafe fn init_alloc() {
    // Initialize the allocator BEFORE you use it.
    let start = heap_bottom();
    let size = 128; // in bytes
    ALLOCATOR.init(start, size)
}

#[riscv_rt::entry]
fn main() -> ! {
    unsafe { init_alloc() }

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

    let mut data = None;
    loop {
        if let Ok(event) = net_reader.poll_data() {
            match event {
                Event::Connected { .. } => {}
                Event::Closed { link_id } => {
                    uprintln!("Closed {}", link_id);
                }
                Event::DataAvailable { mut reader, .. } => {
                    let mut packet_reader = PacketReader::default();
                    let msg_len = packet_reader.read_message_len(&mut reader);
                    let msg = packet_reader.read_message(&mut reader, msg_len).unwrap();

                    match msg {
                        IncomingMessage::GetInfo => {}
                        IncomingMessage::AddImage {
                            refresh_rate,
                            strip_len,
                            reader,
                        } => {
                            data = Some((refresh_rate, strip_len, reader.len()));
                        }
                        IncomingMessage::ClearImages => {}
                        IncomingMessage::Info(_) => {}
                        IncomingMessage::Error(_) => {}
                    };

                    for _ in reader {}
                }
            }
        }

        if let Some((refresh_rate, strip_len, reader_len)) = data.take() {
            uprintln!(
                "Got image: refresh_rate: {}, strip_len: {}, reader_len: {}",
                refresh_rate,
                strip_len,
                reader_len
            );
        }
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

#[alloc_error_handler]
fn oom(layout: Layout) -> ! {
    uprintln!("OOM: {:?}", layout);

    loop {
        continue;
    }
}
