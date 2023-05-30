#![no_std]
#![no_main]

use embedded_storage::{ReadStorage, Storage};
use esp_backtrace as _;
use esp_println::{logger::init_logger, println};
use esp_storage::FlashStorage;
use hal::{
    clock::{ClockControl, CpuClock},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc,
};

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

    let mut bytes = [0u8; 32];

    let mut flash = FlashStorage::new();

    let flash_addr = 0x9000;
    println!("Flash size = {}", flash.capacity());

    flash.read(flash_addr, &mut bytes).unwrap();
    println!("Read from {:x}:  {:02x?}", flash_addr, &bytes[..32]);

    bytes[0x00] = bytes[0x00].wrapping_add(1);
    bytes[0x01] = bytes[0x01].wrapping_add(2);
    bytes[0x02] = bytes[0x02].wrapping_add(3);
    bytes[0x03] = bytes[0x03].wrapping_add(4);
    bytes[0x04] = bytes[0x04].wrapping_add(1);
    bytes[0x05] = bytes[0x05].wrapping_add(2);
    bytes[0x06] = bytes[0x06].wrapping_add(3);
    bytes[0x07] = bytes[0x07].wrapping_add(4);

    flash.write(flash_addr, &bytes).unwrap();
    println!("Written to {:x}: {:02x?}", flash_addr, &bytes[..32]);

    let mut reread_bytes = [0u8; 32];
    flash.read(flash_addr, &mut reread_bytes).unwrap();
    println!("Read from {:x}:  {:02x?}", flash_addr, &reread_bytes[..32]);

    loop {
        core::hint::spin_loop();
    }
}
