//! Embassy SPI
//!
//! Depending on your target and the board you are using you have to change the
//! pins.
//!
//! Connect MISO and MOSI pins to see the outgoing data is read as incoming
//! data.
//!
//! The following wiring is assumed:
//! SCLK => GPIO0
//! MISO => GPIO2
//! MOSI => GPIO4
//! CS   => GPIO5

//% CHIPS: esp32 esp32c2 esp32c3 esp32c6 esp32h2 esp32s2 esp32s3
//% FEATURES: embassy embassy-generic-timers

#![no_std]
#![no_main]

use cyberpixie::ws2812_spi::{self, size_of_line};
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::{
    dma::*,
    dma_buffers,
    prelude::*,
    spi::{
        master::{Config, Spi, SpiDmaBus},
        SpiMode,
    },
    timer::timg::TimerGroup,
    Async,
};
use log::info;
use smart_leds::{brightness, RGB8};

const NUM_LEDS: usize = 48;
const LED_BUF_LEN: usize = size_of_line(NUM_LEDS);
const LED_BRIGHTNESS: u8 = 64;

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

async fn spi_task(spi: &mut SpiDmaBus<'_, Async>) {
    esp_println::println!("Cleaning led");
    for _ in 0..100 {
        const BLANK_LINE_BUF: usize = size_of_line(72);
        let blank = ws2812_spi::make_row::<BLANK_LINE_BUF>([RGB8::default(); 72]);
        spi.write_async(&blank).await.unwrap();
    }
    esp_println::println!("Rainbow example is ready to start");

    loop {
        let counts = 10_000;
        let mut total_render_time = 0;

        esp_println::println!("Starting benchmark cycle");
        esp_println::println!("");

        for j in 0..counts {
            let now = Instant::now();

            spi.write_async(&ws2812_spi::make_row::<LED_BUF_LEN>(brightness(
                (0..NUM_LEDS)
                    .map(|i| wheel((((i * 256) as u16 / NUM_LEDS as u16 + j as u16) & 255) as u8)),
                LED_BRIGHTNESS,
            )))
            .await
            .unwrap();

            let elapsed = now.elapsed().as_micros();
            total_render_time += elapsed;
        }

        let line_render_time = total_render_time as f32 / counts as f32;

        esp_println::println!("-> Num leds {}", NUM_LEDS);
        esp_println::println!("-> Total rendering time {total_render_time}us");
        esp_println::println!("-> per line: {line_render_time}us");
        esp_println::println!(
            "-> Average frame rendering frame rate is {}Hz",
            1_000_000f32 / line_render_time
        );
        esp_println::println!(
            "-> Average frame rendering pixel rate is {}Hz",
            (1_000_000f32 / line_render_time) * NUM_LEDS as f32
        );
    }
}

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    esp_println::println!("Init!");
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    let sclk = peripherals.GPIO6;
    let miso = peripherals.GPIO2;
    let mosi = peripherals.GPIO7;
    let cs = peripherals.GPIO10;

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32_000);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let mut spi = Spi::new_with_config(
        peripherals.SPI2,
        Config {
            frequency: 6000u32.kHz(),
            mode: SpiMode::Mode0,
            ..Config::default()
        },
    )
    .with_sck(sclk)
    .with_mosi(mosi)
    .with_miso(miso)
    .with_cs(cs)
    .with_dma(dma_channel.configure(false, DmaPriority::Priority0))
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    spi_task(&mut spi).await;
}
