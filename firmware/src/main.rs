#![no_std]
#![no_main]

use core::{iter::repeat, panic::PanicInfo, time::Duration};

use cyberpixie::{
    leds::SmartLedsWrite,
    proto::{DeviceRole, Handshake, Service},
    stdout::uprintln,
    time::Microseconds,
    App, Storage,
};
use cyberpixie_firmware::{
    config::{
        ESP32_SERIAL_PORT_CONFIG, SD_MMC_SPI_FREQUENCY, SD_MMC_SPI_TIMEOUT, SERIAL_PORT_CONFIG,
        SOCKET_TIMEOUT, STRIP_LEDS_COUNT, TIMER_TICK_FREQUENCY,
    },
    init_stdout, irq, new_async_timer,
    splash::WanderingLight,
    time::McycleClock,
    NetworkConfig, NextImageBtn, StorageImpl, TransportImpl, BLUE_LED, MAGENTA_LED, RED_LED,
};
use embedded_hal::digital::v2::OutputPin;
use embedded_sdmmc::Block;
use esp8266_softap::{Adapter, ADAPTER_BUF_CAPACITY};
use gd32vf103xx_hal::{
    pac::{self},
    prelude::*,
    serial::Serial,
    spi::{Spi, MODE_0},
    timer::Timer,
    watchdog::FreeWatchdog,
};
use riscv::interrupt;
use smart_leds::RGB8;
use ws2812_spi::Ws2812;

#[export_name = "TIMER1"]
unsafe fn handle_uart1_interrupt() {
    irq::handle_usart1_update()
}

async fn run_main_loop(dp: pac::Peripherals) -> ! {
    let mut rcu = dp.RCU.configure().sysclk(108.mhz()).freeze();
    let mut afio = dp.AFIO.constrain(&mut rcu);

    // Stdout initialization step.
    let gpioa = dp.GPIOA.split(&mut rcu);
    let (usb_tx, mut _usb_rx) = {
        let tx = gpioa.pa9.into_alternate_push_pull();
        let rx = gpioa.pa10.into_floating_input();

        let serial = Serial::new(dp.USART0, (tx, rx), SERIAL_PORT_CONFIG, &mut afio, &mut rcu);
        serial.split()
    };
    init_stdout(usb_tx);

    let mut timer = new_async_timer(Timer::timer0(dp.TIMER0, 1.khz(), &mut rcu));
    timer.delay(Duration::from_secs(2)).await;
    uprintln!();
    uprintln!("Welcome to Cyberpixie serial console!");

    // WS2812 LED strip initialization step.
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

    // Resetting esp8266 module.
    strip.write(RED_LED.iter().copied()).ok();
    uprintln!("Enabling esp8266 serial device");

    let mut esp_en = gpioa.pa4.into_push_pull_output();
    esp_en.set_low().ok();
    timer.delay(Duration::from_secs(1)).await;
    esp_en.set_high().ok();
    timer.delay(Duration::from_secs(1)).await;
    uprintln!("esp8266 device has been enabled");

    // Initializing a UART communication with the esp8266 module.
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
        timer: Timer::timer1(dp.TIMER1, TIMER_TICK_FREQUENCY, &mut rcu),
        watchdog: FreeWatchdog::new(dp.FWDGT),
    });
    uprintln!("esp32 serial communication port configured.");

    // Initializing SD card storage.
    let clock = McycleClock::new(&rcu.clocks);
    let device = {
        let gpiob = dp.GPIOB.split(&mut rcu);
        let spi = Spi::spi1(
            dp.SPI1,
            (
                gpiob.pb13.into_alternate_push_pull(),
                gpiob.pb14.into_floating_input(),
                gpiob.pb15.into_alternate_push_pull(),
            ),
            MODE_0,
            SD_MMC_SPI_FREQUENCY,
            &mut rcu,
        );

        let mut cs = gpiob.pb12.into_push_pull_output();
        cs.set_low().unwrap();

        let mut device = embedded_sdmmc::SdMmcSpi::new(spi, cs, clock, SD_MMC_SPI_TIMEOUT);
        device.init().unwrap();
        device
    };
    let storage = StorageImpl::open(device).unwrap();

    #[cfg(feature = "reset_on_start")]
    storage
        .reset(
            cyberpixie_firmware::config::APP_CONFIG,
            cyberpixie_firmware::config::NETWORK_CONFIG,
        )
        .unwrap();

    let cfg = storage.load_config().unwrap();
    uprintln!("Total images count: {}", storage.images_count());

    if !cfg.safe_mode {
        uprintln!("Showing splash...");
        let splash = WanderingLight::<STRIP_LEDS_COUNT>::default();
        for (ticks, line) in splash {
            timer.start(Microseconds(ticks));
            strip.write(core::array::IntoIter::new(line)).ok();
            timer.wait().await;
        }
        uprintln!("Splash has been showed.");
    }

    // Network initialization step.
    strip.write(MAGENTA_LED.iter().copied()).ok();
    let (socket, role) = {
        let mut blocks = [Block::new()];
        let net_config = storage.network_config(&mut blocks).unwrap();
        uprintln!("Network config is {:?}", net_config);

        let role = net_config.device_role();
        let socket = net_config
            .establish(Adapter::new(esp_rx, esp_tx, clock, SOCKET_TIMEOUT).unwrap())
            .unwrap();
        (socket, role)
    };
    uprintln!("Device IP address is {}", socket.ap_address());

    let device_id = cyberpixie_firmware::device_id();
    let mut network = Service::new(TransportImpl::new(socket, clock), ADAPTER_BUF_CAPACITY);
    if role == DeviceRole::Secondary {
        uprintln!("Exchanging hanshakes with the main device");
        // In order for the main device to know about the existence of the second one,
        // the secondary device has to send a handshake message to the main one.
        network
            .handshake(
                NetworkConfig::LINK_ID,
                Handshake {
                    device_id,
                    role,
                    group_id: None,
                },
            )
            .unwrap()
            .unwrap();
    }

    uprintln!("Network is successfully configured.",);
    strip.write(BLUE_LED.iter().copied()).ok();

    let mut events = NextImageBtn::new(gpioa.pa8.into_pull_down_input());
    let app = App {
        role,
        device_id,

        network: &mut network,
        timer,
        storage: &storage,
        strip,
        events: &mut events,
    };
    app.run().await
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

    unsafe {
        interrupt::disable();
    }

    loop {
        use core::sync::atomic::{self, Ordering};
        atomic::compiler_fence(Ordering::SeqCst);
    }
}
