#![no_std]
#![no_main]

use core::{iter::repeat, panic::PanicInfo, sync::atomic, time::Duration};

use atomic::Ordering;
use cyberpixie::{
    leds::SmartLedsWrite,
    proto::{DeviceRole, Handshake, Message, Service, SimpleMessage},
    stdio::uprintln,
};
use cyberpixie_firmware::{
    config::{ESP32_SERIAL_PORT_CONFIG, SERIAL_PORT_CONFIG, STRIP_LEDS_COUNT},
    device_id, irq, new_async_timer, transport, BLUE_LED, MAGENTA_LED, RED_LED,
};
use embedded_hal::digital::v2::OutputPin;
use esp8266_softap::{net::SocketAddr, softap::JoinApConfig, Adapter, ADAPTER_BUF_CAPACITY};
use gd32vf103xx_hal::{
    pac::{self},
    prelude::*,
    serial::{Event as SerialEvent, Serial},
    spi::Spi,
    timer::Timer,
};
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

    esp_en.set_low().ok();
    timer.delay(Duration::from_secs(1)).await;

    esp_en.set_high().ok();
    timer.delay(Duration::from_secs(2)).await;
    uprintln!("esp32 device has been enabled");

    let (esp_tx, esp_rx) = {
        let tx = gpioa.pa2.into_alternate_push_pull();
        let rx = gpioa.pa3.into_floating_input();

        let serial = Serial::new(
            dp.USART1,
            (tx, rx),
            ESP32_SERIAL_PORT_CONFIG,
            &mut afio,
            &mut rcu,
        );
        serial.split()
    };

    let esp_rx = irq::init_interrupts(irq::Usart1 {
        rx: esp_rx,
        timer: Timer::timer1(dp.TIMER1, 15.khz(), &mut rcu),
    });
    uprintln!("esp32 serial communication port configured.");

    let adapter = Adapter::new(esp_rx, esp_tx).unwrap();
    uprintln!("Adapter created");

    let link_id = 0;
    let socket = JoinApConfig {
        ssid: "cyberpixie_3941434633637FFFFFFFF",
        password: "12345678",
        link_id,
        address: SocketAddr::new([192, 168, 4, 1].into(), 333),
    }
    .join(adapter)
    .unwrap();

    strip.write(MAGENTA_LED.iter().copied()).ok();
    uprintln!("Sending handshake to the master device...");

    let handshake = Handshake {
        device_id: device_id(),
        group_id: None,
        role: DeviceRole::Slave,
    };

    let mut service = Service::new(TransportImpl::new(socket), ADAPTER_BUF_CAPACITY);
    let resp = service.handshake(link_id, handshake).unwrap();

    uprintln!("Got handshake: {:?}", resp);
    strip.write(BLUE_LED.iter().copied()).ok();

    let mut image_index = 0;
    loop {
        let event = service.next_event().await.unwrap();

        let (address, message) = if let Some(message) = event.message() {
            message
        } else {
            continue;
        };

        let response = match message {
            Message::HandshakeRequest(msg) => {
                uprintln!("Handle HandshakeRequest: {:?}", msg);

                Some(SimpleMessage::HandshakeResponse(handshake))
            }

            Message::AddImage { bytes, .. } => {
                for _ in bytes {}

                uprintln!("Handle AddImage");

                image_index += 1;
                Some(SimpleMessage::ImageAdded { index: image_index })
            }

            Message::ShowImage { .. } => {
                uprintln!("Handle ShowImage");

                Some(SimpleMessage::Ok)
            }

            Message::ClearImages => {
                uprintln!("Handle ClearImages");

                image_index = 0;
                Some(SimpleMessage::Ok)
            }

            Message::GetInfo => unimplemented!(),
            _ => None,
        };
        service.confirm_message(address).unwrap();

        if let Some(message) = response {
            service.send_message(address, message).unwrap();
        }
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
    uprintln!();
    uprintln!("The firmware panicked!");
    uprintln!("- {}", info);

    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
