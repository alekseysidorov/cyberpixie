#![cfg_attr(not(test), no_std)]

pub mod adapter;

use core::fmt;

#[cfg(test)]
mod tests;

pub struct ClRf;

impl fmt::Display for ClRf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\r\n")
    }
}
