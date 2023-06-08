//! WS2812b render example on top of the Embassy SPI
//!
//! Following pins are used:
//! SCLK    GPIO6
//! MISO    GPIO2
//! MOSI    GPIO7
//! CS      GPIO10
//!
//! This example demonstrates the overhead of additional load on a frame rendering time.

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use cyberpixie_esp32c3::{wheel, ws2812_spi, SpiType};
use cyberpixie_esp_common::singleton;
use embassy_executor::Executor;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Rtc,
};
use smart_leds::{brightness, RGB8};
use ws2812_async::Ws2812;

const NUM_LEDS: usize = 24;

#[embassy_executor::task]
async fn spi_task(spi: &'static mut SpiType<'static>) {
    const LED_BUF_LEN: usize = 12 * NUM_LEDS;

    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);

    ws.write(core::iter::repeat(RGB8::default()).take(NUM_LEDS))
        .await
        .unwrap();
    loop {
        let counts = 1024;
        let mut total_render_time = 0;

        for j in 0..counts {
            let now = Instant::now();

            let data = (0..NUM_LEDS)
                .map(|i| wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8));
            ws.write(brightness(data, 16)).await.unwrap();

            let elapsed = now.elapsed().as_micros();
            total_render_time += elapsed;

            Timer::after(Duration::from_micros(100)).await;
        }

        let line_render_time = total_render_time as f32 / counts as f32;
        println!("-> Total rendering time {total_render_time}us");
        println!("-> per line: {line_render_time}us");
        println!(
            "-> Average frame rendering frame rate is {}Hz",
            1_000_000f32 / line_render_time
        );
    }
}

#[embassy_executor::task]
async fn dummy_task(_nope: &'static mut ()) {
    loop {
        // Imitate cpu bound task
        for _ in 0..100500 {
            core::hint::spin_loop();
        }

        Timer::after(Duration::from_millis(50)).await;
    }
}

#[entry]
fn main() -> ! {
    esp_println::println!("Init!");

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

    let spi = singleton!(ws2812_spi(
        peripherals.SPI2,
        peripherals.GPIO,
        peripherals.IO_MUX,
        peripherals.DMA,
        &mut system.peripheral_clock_control,
        &clocks
    ));

    let dummy = singleton!(());

    // Initialize and run an Embassy executor.
    embassy::init(&clocks, timer_group0.timer0);
    let executor = singleton!(Executor::new());
    executor.run(|spawner| {
        spawner.spawn(spi_task(spi)).ok();
        spawner.spawn(dummy_task(dummy)).ok();
    })
}
