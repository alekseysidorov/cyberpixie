//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]
#![feature(type_alias_impl_trait)]

use hal::{
    dma::{ChannelRx, ChannelTx},
    gdma::{Channel0RxImpl, Channel0TxImpl, SuitablePeripheral0},
    spi::{dma::SpiDma, FullDuplexMode},
};
use smart_leds::RGB8;

pub const NUM_LEDS: usize = 24;

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
