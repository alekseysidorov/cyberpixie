#![no_std]
#![allow(incomplete_features)]
#![feature(generic_associated_types)]

pub use self::time::TimerImpl;

pub mod config;
pub mod network;
pub mod splash;
pub mod storage;
pub mod time;

pub fn device_id() -> [u32; 4] {
    let mut id = [0; 4];
    id[1..].copy_from_slice(gd32vf103xx_hal::signature::device_id());
    id
}

#[derive(Clone, Copy)]
pub struct LinesIter<I, const N: usize> {
    iter: I,
    lines_remaining: usize,
}

impl<I, const N: usize> LinesIter<I, N>
where
    I: Iterator + ExactSizeIterator,
    I::Item: Default + Copy,
{
    pub fn new(iter: I) -> Self {
        assert_eq!(iter.len() % N, 0);
        let lines_remaining = iter.len() / N;
        Self {
            iter,
            lines_remaining,
        }
    }
}

impl<I, const N: usize> Iterator for LinesIter<I, N>
where
    I: Iterator,
    I::Item: Default + Copy,
{
    type Item = [I::Item; N];

    fn next(&mut self) -> Option<Self::Item> {
        if self.lines_remaining == 0 {
            return None;
        }

        let mut line = [I::Item::default(); N];
        (0..line.len()).for_each(|idx| line[idx] = self.iter.next().unwrap());
        self.lines_remaining -= 1;
        Some(line)
    }
}
