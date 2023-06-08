#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_app::{core::proto::types::Hertz, Configuration, Storage};
use cyberpixie_embedded_storage::StorageImpl;
use cyberpixie_esp32c3::{DEFAULT_MEMORY_LAYOUT};
use cyberpixie_esp_common::singleton;
use esp_backtrace as _;
use esp_println::logger::init_logger;
use esp_storage::FlashStorage;
use hal::{
    clock::{ClockControl, CpuClock},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc,
};

const RAW_IMAGE: &[u8] = include_bytes!("../../../assets/nyan_cat_24.raw").as_slice();

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    // Initialize peripherals
    let peripherals = Peripherals::take();
    let mut system = peripherals.SYSTEM.split();

    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
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

    // Disable watchdog timers
    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    let mut storage = StorageImpl::init(
        Configuration::default(),
        FlashStorage::new(),
        DEFAULT_MEMORY_LAYOUT,
        singleton!([0_u8; 512]),
    )
    .unwrap();
    storage.add_image(Hertz(50), RAW_IMAGE).unwrap();
    log::info!(
        "Image written: total count is {}",
        storage.images_count().unwrap()
    );

    loop {
        core::hint::spin_loop();
    }
}
