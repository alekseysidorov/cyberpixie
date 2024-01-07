#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_esp32s3::{app_task, ws2812_spi};
use cyberpixie_esp_common::{singleton, wifi::WifiManager};
use embassy_executor::Executor;
use esp_backtrace as _;
use esp_println::logger::init_logger;
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rng, Rtc,
};

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();

    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze();

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
    let timer = timer_group1.timer0;
    let (wifi, _bluetooth) = peripherals.RADIO.split();

    let wifi_manager = WifiManager::new(
        cyberpixie_esp_common::wifi::Mode::default(),
        wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    );

    // Initialize embassy async reactor.
    embassy::init(&clocks, timer_group0.timer0);

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
        let (framebuffer, rendering_handle) = cyberpixie_esp_common::render::must_spawn(spawner);
        let stack = wifi_manager.must_spawn(spawner);

        spawner.must_spawn(app_task(stack, rendering_handle));
        spawner.must_spawn(cyberpixie_esp32s3::render_task(spi, framebuffer));
    })
}
