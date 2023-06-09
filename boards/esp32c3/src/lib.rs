//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]
#![feature(async_fn_in_trait, type_alias_impl_trait)]

use cyberpixie_app::core::{proto::types::Hertz, MAX_STRIP_LEN};
use cyberpixie_esp_common::{
    render::{Frame, StaticReceiver, QUEUE_LEN},
    singleton,
};
pub use cyberpixie_esp_common::{
    BoardImpl, NetworkSocketImpl, NetworkStackImpl, DEFAULT_MEMORY_LAYOUT,
};
use embassy_time::{Duration, Instant, Timer};
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

#[embassy_executor::task]
pub async fn render_task(
    spi: &'static mut SpiType<'static>,
    receiver: StaticReceiver<Frame, QUEUE_LEN>,
) {
    const LED_BUF_LEN: usize = 12 * MAX_STRIP_LEN;

    // Initialize and cleanup a LEN strip.
    let mut ws: Ws2812<_, LED_BUF_LEN> = Ws2812::new(spi);
    ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
        .await
        .unwrap();

    // Default frame duration
    let mut rate = Hertz(500);
    let mut frame_duration = Duration::from_hz(rate.0 as u64);

    let mut total_render_time = 0;
    let mut dropped_frames = 0;
    let mut counts = 0;
    let mut max_render_time = 0;
    loop {
        let now = Instant::now();
        match receiver.recv().await {
            // Received a new picture frame rate, we should update a refresh period and wait for
            // a short time until the frames queue will be fill.
            Frame::UpdateRate(new_rate) => {
                rate = new_rate;
                frame_duration = Duration::from_hz(rate.0 as u64);
                Timer::after(frame_duration * QUEUE_LEN as u32 * 2).await;
            }

            Frame::Line(line) => {
                ws.write(line.into_iter()).await.unwrap();
                let elapsed = now.elapsed();

                total_render_time += elapsed.as_micros();
                if elapsed <= frame_duration {
                    let next_frame_time = now + frame_duration;
                    Timer::at(next_frame_time).await;
                } else {
                    dropped_frames += 1;
                }
                max_render_time = core::cmp::max(max_render_time, elapsed.as_micros());
                counts += 1;
            }

            Frame::Clear => {
                ws.write(core::iter::repeat(RGB8::default()).take(MAX_STRIP_LEN))
                    .await
                    .unwrap();
                // Reset rendering stats.
                dropped_frames = 0;
                total_render_time = 0;
                max_render_time = 0;
                counts = 0;
            }
        };

        if counts == 10_000 {
            let line_render_time = total_render_time as f32 / counts as f32;
            log::info!("-> Refresh rate {rate}hz");
            log::info!("-> Total rendering time {total_render_time}us");
            log::info!("-> per line: {line_render_time}us");
            log::info!("-> max: {max_render_time}us");
            log::info!(
                "-> Average frame rendering frame rate is {}Hz",
                1_000_000f32 / line_render_time
            );
            log::info!(
                "-> dropped frames: {dropped_frames} [{}%]",
                dropped_frames as f32 * 100_f32 / counts as f32
            );

            dropped_frames = 0;
            total_render_time = 0;
            counts = 0;
        }
    }
}
