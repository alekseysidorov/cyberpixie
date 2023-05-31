//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]
#![feature(type_alias_impl_trait)]

use cyberpixie_embedded_storage::MemoryLayout;
use hal::{
    clock::Clocks,
    dma::{ChannelRx, ChannelTx, DmaPriority},
    gdma::{Channel0RxImpl, Channel0TxImpl, Gdma, SuitablePeripheral0},
    peripherals::{DMA, GPIO, IO_MUX, SPI2},
    prelude::*,
    spi::{dma::SpiDma, FullDuplexMode, SpiMode},
    system::PeripheralClockControl,
    Spi, IO,
};
use smart_leds::RGB8;

/// Default memory layout of internal Flash storage.
pub const DEFAULT_MEMORY_LAYOUT: MemoryLayout = MemoryLayout {
    base: 0x9000,
    size: 0x199000,
};

/// Initializes SPI for the ws2812 async driver on the pin 7.
pub fn ws2812_spi(
    spi: SPI2,
    gpio: GPIO,
    io_mux: IO_MUX,
    dma: DMA,
    peripheral_clock_control: &mut PeripheralClockControl,
    clocks: &Clocks,
) -> SpiType<'static> {
    hal::interrupt::enable(
        hal::peripherals::Interrupt::DMA_CH0,
        hal::interrupt::Priority::Priority1,
    )
    .unwrap();

    let io = IO::new(gpio, io_mux);
    let sclk = io.pins.gpio6;
    let miso = io.pins.gpio2;
    let mosi = io.pins.gpio7;
    let cs = io.pins.gpio10;

    let dma = Gdma::new(dma, peripheral_clock_control);
    let dma_channel = dma.channel0;

    let descriptors = singleton!([0u32; 8 * 3]);
    let rx_descriptors = singleton!([0u32; 8 * 3]);

    Spi::new(
        spi,
        sclk,
        mosi,
        miso,
        cs,
        3800u32.kHz(),
        SpiMode::Mode0,
        peripheral_clock_control,
        clocks,
    )
    .with_dma(dma_channel.configure(
        false,
        descriptors,
        rx_descriptors,
        DmaPriority::Priority0,
    ))
}

/// Input a value 0 to 255 to get a color value
/// The colors are a transition r - g - b - back to r.
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

/// SPI type using by the ws2812 driver.
pub type SpiType<'d> = SpiDma<
    'd,
    hal::peripherals::SPI2,
    ChannelTx<'d, Channel0TxImpl, hal::gdma::Channel0>,
    ChannelRx<'d, Channel0RxImpl, hal::gdma::Channel0>,
    SuitablePeripheral0,
    FullDuplexMode,
>;

/// Creates a singleton value in the static memory and returns a mutable reference.
#[macro_export]
macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: static_cell::StaticCell<T> = static_cell::StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}
