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

use cyberpixie_esp32c3::{
    create_ws2812_spi,
    ws2812_spi::{self, size_of_line},
    AsyncSpi,
};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, channel};
use embassy_time::Instant;
use embedded_hal_async::spi::SpiBus;
use esp32c3_hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::{_esp_hal_system_SystemExt, main, entry},
};
use esp_backtrace as _;
use esp_println::println;
use smart_leds::{brightness, RGB8};
use static_cell::make_static;

const NUM_LEDS: usize = 48;

type Receiver<T> = channel::Receiver<'static, NoopRawMutex, T, 2>;

const LED_ROW_BUF_LEN: usize = size_of_line(NUM_LEDS);
type LedRow = [u8; LED_ROW_BUF_LEN];

#[embassy_executor::task]
async fn spi_task(rows: Receiver<LedRow>, spi: &'static mut AsyncSpi) {
    println!("Cleaning led");
    for _ in 0..100 {
        const BLANK_LINE_BUF: usize = size_of_line(72);
        let blank = ws2812_spi::make_row::<BLANK_LINE_BUF>([RGB8::default(); 72]);
        spi.write(&blank).await.unwrap();
    }
    println!("Rainbow example is ready to start");

    loop {
        let counts = 10_000;
        let mut total_render_time = 0;

        println!("Starting benchmark cycle");
        println!();

        for _ in 0..counts {
            let now = Instant::now();

            let data = rows.receive().await;
            spi.write(&data).await.unwrap();

            let elapsed = now.elapsed().as_micros();
            total_render_time += elapsed;
        }

        let line_render_time = total_render_time as f32 / counts as f32;

        println!("-> Num leds {}", NUM_LEDS);
        println!("-> Total rendering time {total_render_time}us");
        println!("-> per line: {line_render_time}us");
        println!(
            "-> Average frame rendering frame rate is {}Hz",
            1_000_000f32 / line_render_time
        );
        println!(
            "-> Average frame rendering pixel rate is {}Hz",
            (1_000_000f32 / line_render_time) * NUM_LEDS as f32
        );
    }
}

/// Input a value 0 to 255 to get a color value
/// The colors are a transition r - g - b - back to r.
#[inline]
pub fn wheel(mut wheel_pos: u8) -> RGB8 {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        return (255 - wheel_pos * 3, 0, wheel_pos * 3).into();
    }
    if wheel_pos < 170 {
        wheel_pos -= 85;
        return (0, wheel_pos * 3, 255 - wheel_pos * 3).into();
    }
    wheel_pos -= 170;
    (wheel_pos * 3, 255 - wheel_pos * 3, 0).into()
}

#[main]
async fn main(spawner: Spawner) {
    esp_println::println!("Init!");

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    embassy::init(
        &clocks,
        esp32c3_hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks).timer0,
    );

    let spi = make_static!(create_ws2812_spi(
        peripherals.SPI2,
        peripherals.GPIO,
        peripherals.IO_MUX,
        peripherals.DMA,
        &clocks
    ));

    let ch = make_static!(channel::Channel::new());

    spawner.must_spawn(spi_task(ch.receiver(), spi));

    loop {
        for j in 0..1000 {
            let line = ws2812_spi::make_row(brightness(
                (0..NUM_LEDS)
                    .map(|i| wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8)),
                16,
            ));

            ch.send(line).await;
        }
    }
    // spi_task(spi).await;
}
