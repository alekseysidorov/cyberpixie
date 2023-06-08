#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_app::{
    asynch::App,
    core::{proto::types::Hertz, MAX_STRIP_LEN},
};
use cyberpixie_esp32c3::{singleton, wheel, BoardImpl, SpiType};
use embassy_executor::Executor;
use embassy_net::{Config, Ipv4Address, Ipv4Cidr, Stack, StaticConfig};
use embassy_time::{Duration, Instant, Timer};
use embedded_svc::wifi::{AccessPointConfiguration, Configuration, Wifi};
use esp_backtrace as _;
use esp_println::logger::init_logger;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiMode, WifiState};
use hal::{
    clock::{ClockControl, CpuClock},
    dma::DmaPriority,
    embassy,
    gdma::Gdma,
    peripherals::Peripherals,
    prelude::*,
    spi::SpiMode,
    systimer::SystemTimer,
    timer::TimerGroup,
    Rng, Rtc, Spi, IO,
};
use smart_leds::{brightness, RGB8};
use ws2812_async::Ws2812;

const NUM_LEDS: usize = 24;

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

    hal::interrupt::enable(
        hal::peripherals::Interrupt::DMA_CH0,
        hal::interrupt::Priority::Priority1,
    )
    .unwrap();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let sclk = io.pins.gpio6;
    let miso = io.pins.gpio2;
    let mosi = io.pins.gpio7;
    let cs = io.pins.gpio10;

    let dma = Gdma::new(peripherals.DMA, &mut system.peripheral_clock_control);
    let dma_channel = dma.channel0;

    let descriptors = singleton!([0u32; 8 * 3]);
    let rx_descriptors = singleton!([0u32; 8 * 3]);

    let spi = singleton!(Spi::new(
        peripherals.SPI2,
        sclk,
        mosi,
        miso,
        cs,
        3800u32.kHz(),
        SpiMode::Mode0,
        &mut system.peripheral_clock_control,
        &clocks,
    )
    .with_dma(dma_channel.configure(
        false,
        descriptors,
        rx_descriptors,
        DmaPriority::Priority0,
    )));

    // Spawn Embassy executor
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        // Wifi Network.
        spawner.spawn(connection(controller)).ok();
        spawner.spawn(net_task(stack)).unwrap();
        spawner.spawn(task(stack)).ok();
        // LED Render.
        spawner.spawn(led_render_task(spi)).unwrap();
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
async fn task(stack: &'static Stack<WifiDevice<'static>>) {
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Network config is {:?}", stack.config());

    let board = BoardImpl::new(stack);
    let app = App::new(board).expect("Unable to create a cyberpixie application");
    app.run().await.expect("Application execution failed");
}

#[embassy_executor::task]
async fn led_render_task(spi: &'static mut SpiType<'static>) {
    const LED_BUF_LEN: usize = 12 * MAX_STRIP_LEN;

    let rate = Hertz(500);

    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);
    // Cleanup strip
    ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
        .await
        .unwrap();

    let frame_duration = Duration::from_hz(rate.0 as u64);

    Timer::after(Duration::from_secs(5)).await;
    log::info!("Start LED strip rendering task with refresh rate: {rate}hz");

    let mut total_render_time = 0;
    let mut dropped_frames = 0;
    let mut counts = 0;
    let mut max_render_time = 0;
    loop {
        let now = Instant::now();

        let line = brightness(
            (0..NUM_LEDS)
                .map(|i| wheel((((i * 256) as u16 / NUM_LEDS as u16 + counts as u16) & 255) as u8)),
            16,
        );
        ws.write(line).await.unwrap();
        let elapsed = now.elapsed();

        total_render_time += elapsed.as_micros();
        if elapsed <= frame_duration {
            let next_frame_time = now + frame_duration;
            Timer::at(next_frame_time).await;
        } else {
            dropped_frames += 1;
            max_render_time = core::cmp::max(max_render_time, elapsed.as_micros());
        }

        counts += 1;
        if counts == 10_000 {
            let line_render_time = total_render_time as f32 / counts as f32;
            log::info!("-> Refresh rate {rate}hz");
            log::info!("-> Total rendering time {total_render_time}us");
            log::info!("-> per line: {line_render_time}us");
            log::info!("-> max: {max_render_time}us");
            log::info!(
                "-> Average frame rendering frame rate is {}Hz",
                1_000_000f32 / line_render_time
            );
            log::info!(
                "-> dropped frames: {dropped_frames} [{}%]",
                dropped_frames as f32 * 100_f32 / counts as f32
            );

            dropped_frames = 0;
            total_render_time = 0;
            counts = 0;
        }
    }
}
