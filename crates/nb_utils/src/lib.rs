#![cfg_attr(not(test), no_std)]

//! A collection of useful `nb` extensions.

use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use futures_util::Stream;

#[cfg(test)]
mod tests;

pub trait NbResultExt<T, E> {
    fn filter<P: FnOnce(&T) -> bool>(self, pred: P) -> Self;

    fn filter_map<U, P: FnOnce(T) -> Option<U>>(self, pred: P) -> nb::Result<U, E>;

    fn expect_ok(self, msg: &str) -> Option<T>;
}

impl<T, E> NbResultExt<T, E> for nb::Result<T, E> {
    fn filter<P: FnOnce(&T) -> bool>(self, pred: P) -> Self {
        match self {
            Ok(value) => {
                if pred(&value) {
                    Ok(value)
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }
            other => other,
        }
    }

    fn filter_map<U, P: FnOnce(T) -> Option<U>>(self, pred: P) -> nb::Result<U, E> {
        match self {
            Ok(value) => {
                if let Some(value) = pred(value) {
                    Ok(value)
                } else {
                    Err(nb::Error::WouldBlock)
                }
            }
            Err(nb::Error::Other(other)) => Err(nb::Error::Other(other)),
            Err(nb::Error::WouldBlock) => Err(nb::Error::WouldBlock),
        }
    }

    #[track_caller]
    fn expect_ok(self, msg: &str) -> Option<T> {
        match self {
            Ok(value) => Some(value),
            Err(nb::Error::WouldBlock) => None,

            _ => panic!("{}", msg),
        }
    }
}

struct UntilOk<T, E, F: FnMut() -> nb::Result<T, E>> {
    poll_fn: F,
}

// TODO why do we need to implement Unpin here?
impl<T, E, F: FnMut() -> nb::Result<T, E> + Unpin> Future for UntilOk<T, E, F> {
    type Output = Result<T, E>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        match (self.poll_fn)() {
            Ok(output) => Poll::Ready(Ok(output)),
            Err(nb::Error::Other(err)) => Poll::Ready(Err(err)),
            Err(nb::Error::WouldBlock) => {
                ctx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl<T, E, F: FnMut() -> nb::Result<T, E> + Unpin> Stream for UntilOk<T, E, F> {
    type Item = Result<T, E>;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.poll(ctx).map(Option::Some)
    }
}

/// Convert a function that returns `nb::Result<T, E>` into a valid but inefficient future. The future will
/// resolve only when the function returns `Ok(T)` or `Err(nb::Error::Other).
pub fn poll_nb_future<T, E, F>(poll_fn: F) -> impl Future<Output = Result<T, E>>
where
    F: FnMut() -> nb::Result<T, E> + Unpin,
{
    UntilOk { poll_fn }
}

pub fn poll_nb_stream<T, E, F>(poll_fn: F) -> impl Stream<Item = Result<T, E>>
where
    F: FnMut() -> nb::Result<T, E> + Unpin,
{
    UntilOk { poll_fn }
}

struct Yeld(bool);

impl Future for Yeld {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0 {
            self.0 = false;
            ctx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub fn yeld_executor() -> impl Future<Output = ()> {
    Yeld(true)
}
