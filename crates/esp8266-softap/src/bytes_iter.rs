use embedded_hal::serial;

use crate::{DataReader, Event};

pub struct BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    data: DataReader<'a, Rx>,
    link_id: usize,
    bytes_remaining: usize,
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

    fn wait_for_next_data(&mut self) -> Result<(), Rx::Error> {
        loop {
            let event = nb::block!(self.data.reader.poll_next_event())?;

            match event {
                Event::DataAvailable { link_id, reader } if link_id == self.link_id => {
                    // FIXME: Rewrite this code without breaking encapsulation rules.
                    self.data.bytes_remaining = reader.bytes_remaining;
                    self.data.read_pos = reader.read_pos;
                    return Ok(());
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

        self.wait_for_next_data()
            .expect("Panic in the bytes reader");
        self.data.next()
    }
}

impl<'a, Rx> ExactSizeIterator for BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
}

impl<'a, Rx> Drop for BytesIter<'a, Rx>
where
    Rx: serial::Read<u8> + 'static,
    Rx::Error: core::fmt::Debug,
{
    fn drop(&mut self) {
        // In order to use the reader further, we must read all of the remaining bytes.
        // Otherwise, the reader will be in an inconsistent state.
        for _ in &mut self.data {}
    }
}
