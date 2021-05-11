#![no_std]
#![no_main]

use core::{
    panic::PanicInfo,
    sync::atomic::{self, Ordering},
};

use cyberpixie_firmware::{
    config::{MAX_LINES_COUNT, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    storage::{ImagesRepository, RgbWriter},
};
use cyberpixie_proto::{types::Hertz, IncomingMessage, PacketReader};
use embedded_hal::{digital::v2::OutputPin, spi::MODE_0};
use esp8266_softap::{Adapter, BytesIter, Event, SoftAp, SoftApConfig};
use gd32vf103xx_hal::{delay::McycleDelay, pac::Peripherals, prelude::*, serial::Serial, spi::Spi};
use heapless::Vec;
use smart_leds::RGB8;
use stdio_serial::uprintln;

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

    const LEN: usize = MAX_LINES_COUNT * STRIP_LEDS_COUNT;
    let mut buf: Vec<RGB8, LEN> = Vec::new();
    let mut rate = Hertz(0);

    loop {
        let event = if let Ok(event) = net_reader.poll_data() {
            event
        } else {
            continue;
        };

        match event {
            Event::Connected { .. } => {}
            Event::Closed { link_id } => {
                uprintln!("closed {}, buf_len: {}", link_id, buf.len());
                images.add_image(buf.iter().copied(), rate).unwrap();
                buf.clear();
                uprintln!("Images count: {}", images.count());
            }
            Event::DataAvailable {
                link_id,
                mut reader,
            } => {
                let mut packet_reader = PacketReader::default();
                let (header_len, payload_len) = packet_reader.read_message_len(&mut reader);

                let bytes = BytesIter::new(link_id, reader, payload_len + header_len);
                let msg = packet_reader.read_message(bytes, header_len).unwrap();

                match msg {
                    IncomingMessage::GetInfo => {}
                    IncomingMessage::AddImage {
                        bytes,
                        refresh_rate,
                        ..
                    } => {
                        rate = refresh_rate;
                        buf.extend(RgbWriter::new(bytes));
                    }
                    IncomingMessage::ClearImages => images = images.reset().unwrap(),
                    IncomingMessage::Info(_) => {}
                    IncomingMessage::Error(_) => {}
                };
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
