//! Cyberpixie application implementation on top of the Embassy framework
//! for the esp32c3 board.

#![no_std]

use hal::{
    dma::{ChannelRx, ChannelTx},
    gdma::{Channel0RxImpl, Channel0TxImpl, SuitablePeripheral0},
    spi::{dma::SpiDma, FullDuplexMode},
};

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
