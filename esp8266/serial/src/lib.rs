#![cfg_attr(not(test), no_std)]

#[cfg(test)]
mod tests;

use core::fmt;

pub struct ClRf;

impl fmt::Display for ClRf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\r\n")
    }
}
