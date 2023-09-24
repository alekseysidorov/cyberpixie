#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]


use cyberpixie_esp_common::ble::run_task;
use embassy_executor::{Executor, _export::StaticCell};
use esp_backtrace as _;
use esp_wifi::{
    initialize, EspWifiInitFor, EspWifiInitialization,
};
use hal::{
    clock::ClockControl, embassy, peripherals::*, prelude::*, radio::Bluetooth,
    systimer::SystemTimer, timer::TimerGroup, Rng,
};

pub type BootButton = hal::gpio::Gpio9<hal::gpio::Input<hal::gpio::PullDown>>;

#[embassy_executor::task]
async fn run(init: EspWifiInitialization, bluetooth: Bluetooth) {
    run_task(init, bluetooth).await
}

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();

    let mut system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    // Async requires the GPIO interrupt to wake futures
    hal::interrupt::enable(
        hal::peripherals::Interrupt::GPIO,
        hal::interrupt::Priority::Priority1,
    )
    .unwrap();

    let (_, bluetooth, ..) = peripherals.RADIO.split();

    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    embassy::init(&clocks, timer_group0.timer0);
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        spawner.must_spawn(run(init, bluetooth));
    });
}
