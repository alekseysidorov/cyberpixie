#![no_std]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

use core::{
    pin::Pin,
    task::{Context, Poll},
};

use cyberpixie::{futures::Stream, HwEvent};
use embedded_hal::digital::v2::InputPin;

pub use self::{storage::StorageImpl, time::new_async_timer, transport::TransportImpl};

pub mod config;
pub mod irq;
pub mod splash;
pub mod storage;
pub mod time;
pub mod transport;

pub fn device_id() -> [u32; 4] {
    let mut id = [0; 4];
    id[1..].copy_from_slice(gd32vf103xx_hal::signature::device_id());
    id
}

pub struct NextImageBtn<T: InputPin> {
    btn: T,
    prev_value: bool,
}

impl<T: InputPin> NextImageBtn<T> {
    pub fn new(btn: T) -> Self {
        let prev_value = btn.is_high().map_err(drop).unwrap();

        Self { btn, prev_value }
    }

    fn is_triggered(&mut self) -> bool {
        let value = self.btn.is_high().map_err(drop).unwrap();

        let is_triggered = !self.prev_value && value;
        self.prev_value = value;
        is_triggered
    }
}

impl<T: InputPin + Unpin> Stream for NextImageBtn<T> {
    type Item = HwEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.get_mut().is_triggered() {
            Poll::Ready(Some(HwEvent::ShowNextImage))
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
