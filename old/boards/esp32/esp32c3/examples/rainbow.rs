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

use cyberpixie_esp32c3::{ws2812_spi, SpiType};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::ClockControl,
    dma::DmaPriority,
    dma_descriptors, embassy,
    gdma::*,
    peripherals::Peripherals,
    prelude::*,
    spi::{
        master::{prelude::*, Spi},
        SpiMode,
    },
    IO,
};
use smart_leds::{brightness, RGB8};
use static_cell::make_static;
use ws2812_async::Ws2x12;

const NUM_LEDS: usize = 36;

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

async fn spi_task(spi: &'static mut SpiType<'static>) {
    const LED_BUF_LEN: usize = 12 * NUM_LEDS;

    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);

    println!("Cleaning led");
    ws.write(core::iter::repeat(RGB8::default()).take(144))
        .await
        .unwrap();
    println!("Rainbow example is ready to start");

    loop {
        let counts = 10_000;
        let mut total_render_time = 0;

        println!("Starting benchmark cycle");
        println!();

        for j in 0..counts {
            let now = Instant::now();

            let data = (0..NUM_LEDS)
                .map(|i| wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8));
            ws.write(brightness(data, 16)).await.unwrap();

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

#[main]
async fn main(_spawner: Spawner) {
    esp_println::println!("Init!");

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    embassy::init(
        &clocks,
        hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks).timer0,
    );

    let spi = make_static!(ws2812_spi(
        peripherals.SPI2,
        peripherals.GPIO,
        peripherals.IO_MUX,
        peripherals.DMA,
        &clocks
    ));

    spi_task(spi).await;
}
