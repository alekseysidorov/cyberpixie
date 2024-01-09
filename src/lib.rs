//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]
#![feature(type_alias_impl_trait)]

pub use esp32c3_hal as hal;
use hal::{
    clock::Clocks,
    dma::DmaPriority,
    dma_descriptors,
    gdma::{Channel0, Gdma},
    peripherals::{DMA, GPIO, IO_MUX, SPI2},
    prelude::*,
    spi::{
        master::{dma::SpiDma, prelude::*, Spi},
        FullDuplexMode, SpiMode,
    },
    IO,
};
use static_cell::make_static;

pub mod ws2812_spi;

// use cyberpixie_app::{core::proto::types::Hertz, App};
// // pub use cyberpixie_esp_common::{
// //     BoardImpl, NetworkSocketImpl, NetworkStackImpl, DEFAULT_MEMORY_LAYOUT,
// // };
// use embassy_net::Stack;
// use embassy_time::{Duration, Timer};
// use esp_println::println;
// use hal::{
//     clock::Clocks,
//     dma::DmaPriority,
//     dma_descriptors,
//     gdma::{Channel0, Gdma},
//     peripherals::{DMA, GPIO, IO_MUX, SPI2},
//     prelude::*,
//     spi::{
//         master::{dma::SpiDma, prelude::*, Spi},
//         FullDuplexMode, SpiMode,
//     },
//     IO,
// };
// use static_cell::make_static;
// use ws2812_async::Ws2812;

/// SPI type using by the ws2812 driver.
pub type AsyncSpi = SpiDma<'static, SPI2, Channel0, FullDuplexMode>;

/// Initializes SPI for the ws2812 async driver on the pin 7.
pub fn create_ws2812_spi(
    spi: SPI2,
    gpio: GPIO,
    io_mux: IO_MUX,
    dma: DMA,
    clocks: &Clocks,
) -> AsyncSpi {
    let io = IO::new(gpio, io_mux);
    let sclk = io.pins.gpio6;
    let miso = io.pins.gpio2;
    let mosi = io.pins.gpio7;
    let cs = io.pins.gpio10;

    let dma = Gdma::new(dma);
    let dma_channel = dma.channel0;

    let dma_descriptors = make_static!(dma_descriptors!(32_000));

    Spi::new(spi, 
        // 3800u32.kHz(),
        5150u32.kHz(), 
        SpiMode::Mode0, clocks)
        .with_pins(Some(sclk), Some(mosi), Some(miso), Some(cs))
        .with_dma(dma_channel.configure(
            false,
            &mut dma_descriptors.0,
            &mut dma_descriptors.1,
            DmaPriority::Priority0,
        ))
}
