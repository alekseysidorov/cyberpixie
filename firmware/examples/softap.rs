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
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    network::DataIter,
    storage::{ImagesRepository, RgbWriter},
};
use cyberpixie_proto::{IncomingMessage, PacketReader};
use embedded_hal::{digital::v2::OutputPin, serial::Read, spi::MODE_0};
use esp8266_softap::{adapter::ReadPart, Adapter, Event, SoftAp, SoftApConfig};
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial, spi::Spi};
use heapless::Vec;
use stdio_serial::{uprint, uprintln};

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

    let mut images = ImagesRepository::open(&mut device).unwrap();
    uprintln!("Total images count: {}", images.count());

    const LEN: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT * 3;
    let mut buf: Vec<u8, LEN> = Vec::new();

    loop {
        if let Ok(event) = net_reader.poll_data() {
            match event {
                Event::Connected { .. } => {}
                Event::Closed { link_id } => {
                    uprintln!("Closed {}", link_id);
                }
                Event::DataAvailable {
                    mut reader,
                    link_id,
                } => {
                    for _ in reader {}
                    continue;

                    let mut packet_reader = PacketReader::default();
                    let msg_len = packet_reader.read_message_len(&mut reader);
                    let msg = packet_reader.read_message(reader, msg_len).unwrap();

                    match msg {
                        IncomingMessage::GetInfo => {}
                        IncomingMessage::AddImage {
                            refresh_rate,
                            strip_len,
                            reader,
                            len,
                        } => {
                            for byte in DataIter::new(link_id, reader, len) {
                                // uprint!("{}", byte as char);
                                // buf.push(byte).unwrap();
                            }

                            // let img_reader = RgbWriter::new(buf.as_slice().into_iter().copied());
                            // images.add_image(img_reader, refresh_rate.hz()).unwrap();
                            // uprintln!("Write image: total images count: {}", images.count());
                        }
                        IncomingMessage::ClearImages => {}
                        IncomingMessage::Info(_) => {}
                        IncomingMessage::Error(_) => {}
                    };
                }
            }
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
