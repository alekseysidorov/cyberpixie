use core::convert::Infallible;

use embedded_hal::serial;
use stdio_serial::uprint;

use crate::{DataReader, Event, ReadPart};

pub struct BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    data: DataReader<'a, Rx>,
    link_id: usize,
    bytes_remaining: usize,
}

fn read_next_packet<'a, Rx>(id: usize, data: &mut DataReader<'a, Rx>)
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    loop {
        let event = nb::block!(data.reader.poll_data()).unwrap();
    }
}

impl<'a, Rx> BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    pub fn new(link_id: usize, data: DataReader<'a, Rx>, len: usize) -> Self {
        Self {
            link_id,
            data,
            bytes_remaining: len,
        }
    }

    fn wait_for_next_data(&mut self) {
        loop {
            let event =
                nb::block!(self.data.reader.poll_data()).expect("Unable to read next packet");

            match event {
                Event::DataAvailable { link_id, reader } if link_id == self.link_id => {
                    // FIXME: Rewrite this code without breaking encapsulation rules.
                    self.data.bytes_remaining = reader.bytes_remaining;
                    self.data.read_pos = reader.read_pos;
                    return;
                }
                // We do not support simultaneous multiple connections.
                Event::DataAvailable { link_id, reader } if link_id != self.link_id => {
                    for _ in reader {}
                }
                _ => {}
            }
        }
    }
}

impl<'a, Rx> Iterator for BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    type Item = u8;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.bytes_remaining, Some(self.bytes_remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes_remaining == 0 {
            return None;
        }

        self.bytes_remaining -= 1;
        if let Some(byte) = self.data.next() {
            return Some(byte);
        }

        self.wait_for_next_data();
        self.data.next()
    }
}

impl<'a, Rx> ExactSizeIterator for BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
}
