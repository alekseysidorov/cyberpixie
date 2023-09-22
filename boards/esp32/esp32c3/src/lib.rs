//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]
#![feature(async_fn_in_trait, type_alias_impl_trait)]

use cyberpixie_app::{core::proto::types::Hertz, App};
use cyberpixie_esp_common::{
    render::{Frame, RenderingHandle, StaticReceiver, QUEUE_LEN},
    singleton,
    wifi::WifiDevice,
};
pub use cyberpixie_esp_common::{
    BoardImpl, NetworkSocketImpl, NetworkStackImpl, DEFAULT_MEMORY_LAYOUT,
};
use embassy_net::Stack;
use embassy_time::{Duration, Timer};
use hal::{
    clock::Clocks,
    dma::DmaPriority,
    gdma::Gdma,
    peripherals::{DMA, GPIO, IO_MUX, SPI2},
    prelude::*,
    spi::{dma::SpiDma, FullDuplexMode, SpiMode},
    system::PeripheralClockControl,
    Spi, IO,
};
use ws2812_async::Ws2812;

/// Max supported frame rate.
pub const MAX_FRAME_RATE: Hertz = Hertz(500);

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

/// SPI type using by the ws2812 driver.
pub type SpiType<'d> = SpiDma<'d, hal::peripherals::SPI2, hal::gdma::Channel0, FullDuplexMode>;

#[embassy_executor::task]
pub async fn render_task(
    spi: &'static mut SpiType<'static>,
    receiver: StaticReceiver<Frame, QUEUE_LEN>,
) {
    cyberpixie_esp_common::render::ws2812_async_render(Ws2812::new(spi), receiver).await;
}

#[embassy_executor::task]
pub async fn app_task(
    stack: &'static Stack<WifiDevice<'static>>,
    rendering_handle: RenderingHandle,
) {
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Network config is {:?}", stack.config_v4());

    let board = BoardImpl::new(stack, rendering_handle);
    let app = App::new(board).expect("Unable to create a cyberpixie application");
    app.run().await.expect("Application execution failed");
}
