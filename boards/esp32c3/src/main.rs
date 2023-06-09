#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_app::App;
use cyberpixie_esp32c3::{ws2812_spi, BoardImpl};
use cyberpixie_esp_common::{render::RenderingHandle, singleton};
use embassy_executor::Executor;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StaticConfig};
use embassy_time::{Duration, Timer};
use embedded_svc::wifi::{AccessPointConfiguration, Configuration, Wifi};
use esp_backtrace as _;
use esp_println::logger::init_logger;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiMode, WifiState};
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng, Rtc,
};

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();

    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    // Disable watchdog timers
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    // Initialize and get Wifi device
    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    esp_wifi::initialize(
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    let (wifi, _bluetooth) = peripherals.RADIO.split();

    // Initialize embassy async reactor.
    embassy::init(&clocks, timer_group0.timer0);

    // Network stack configuration.
    let config = Config::Static(StaticConfig {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 1, 1), 24),
        gateway: Some(Ipv4Address::from_bytes(&[192, 168, 1, 1])),
        dns_servers: Default::default(),
    });
    // FIXME: There is no way to use DHCP in Ap mode at this moment :(
    // let config = Config::Dhcp(Default::default());

    // Initialize the network stack
    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(wifi, WifiMode::Ap);
    let stack = singleton!(Stack::new(
        wifi_interface,
        config,
        singleton!(embassy_net::StackResources::<3>::new()),
        1234
    ));

    // Initialize LED strip SPI
    let spi = singleton!(ws2812_spi(
        peripherals.SPI2,
        peripherals.GPIO,
        peripherals.IO_MUX,
        peripherals.DMA,
        &mut system.peripheral_clock_control,
        &clocks
    ));

    // Spawn Embassy executor
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        // Rendering tasks.
        let (framebuffer, rendering_handle) = cyberpixie_esp_common::render::spawn(spawner);
        spawner.must_spawn(cyberpixie_esp32c3::render_task(spi, framebuffer));
        // Wifi Network.
        spawner.must_spawn(connection(controller));
        spawner.must_spawn(net_task(stack));
        spawner.must_spawn(app_task(stack, rendering_handle));
    })
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    log::info!("start connection task");
    log::info!("Device capabilities: {:?}", controller.get_capabilities());

    log::info!("Waiting for a next wifi state!");
    match esp_wifi::wifi::get_wifi_state() {
        WifiState::ApStart => {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        other => {
            log::info!("Wifi state changed to {other:?}")
        }
    }

    if !matches!(controller.is_started(), Ok(true)) {
        let client_config = Configuration::AccessPoint(AccessPointConfiguration {
            ssid: "esp-wifi".into(),
            ..Default::default()
        });
        controller.set_configuration(&client_config).unwrap();
        log::info!("Starting wifi");
        controller.start().await.unwrap();
        log::info!("Wifi started!");
    }

    log::info!("Wifi connection task finished");
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static>>) {
    stack.run().await
}

#[embassy_executor::task]
async fn app_task(stack: &'static Stack<WifiDevice<'static>>, rendering_handle: RenderingHandle) {
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Network config is {:?}", stack.config());

    let board = BoardImpl::new(stack, rendering_handle);
    let app = App::new(board).expect("Unable to create a cyberpixie application");
    app.run().await.expect("Application execution failed");
}
