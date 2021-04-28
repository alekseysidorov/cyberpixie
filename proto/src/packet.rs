use core::convert::TryInto;

pub const MAX_PACKET_LEN: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Packet {
    pub(crate) size: u16,
    pub(crate) buf: [u8; MAX_PACKET_LEN],
}

impl Packet {
    pub(crate) fn empty() -> Self {
        Self {
            size: 0,
            buf: [0_u8; MAX_PACKET_LEN],
        }
    }

    pub fn read_from(bytes: &[u8]) -> Self {
        assert!(bytes.len() <= MAX_PACKET_LEN);

        let mut packet = Self {
            size: bytes.len() as u16,
            ..Self::empty()
        };
        packet.buf[0..bytes.len()].copy_from_slice(bytes);
        packet
    }

    pub fn write_to(&self, buf: &mut [u8]) -> usize {
        let size = self.size as usize;
        assert!(buf.len() > size + 2);

        buf[0..2].copy_from_slice(&self.size.to_le_bytes());
        buf[2..2 + size].copy_from_slice(self.as_ref());
        size + 2
    }
}

impl AsRef<[u8]> for Packet {
    fn as_ref(&self) -> &[u8] {
        self.buf[0..self.size as usize].as_ref()
    }
}

#[derive(Debug)]
pub struct PacketReader {
    bytes_remaining: usize,
    inner: Packet,
}

impl Default for PacketReader {
    fn default() -> Self {
        Self {
            bytes_remaining: 0,
            inner: Packet {
                size: 0,
                buf: [0_u8; MAX_PACKET_LEN],
            },
        }
    }
}

impl PacketReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_bytes<'a>(&mut self, mut bytes: &'a [u8]) -> Option<(Packet, &'a [u8])> {
        if bytes.is_empty() {
            return None;
        }

        // In this case, we have to read a packet size.
        if self.bytes_remaining == 0 {
            match (self.inner.size, bytes.len()) {
                // We have enough bytes to read a packet length immediately.
                (0, len) if len > 2 => {
                    self.inner.buf[0..2].copy_from_slice(&bytes[0..2]);
                    bytes = &bytes[2..];
                }
                // We have not enough bytes to read length right now.
                (0, len) if len == 1 => {
                    self.inner.buf[0] = bytes[0];
                    self.inner.size += 1;
                    return None;
                }
                // We can read the remaining length byte.
                _ => {
                    self.inner.buf[1] = bytes[0];
                    bytes = &bytes[1..];
                }
            }

            self.inner.size = 0;
            self.bytes_remaining =
                u16::from_le_bytes(self.inner.buf[0..2].try_into().unwrap()) as usize;
        }

        let bytes_to_read = core::cmp::min(self.bytes_remaining, bytes.len());
        let from = self.inner.size as usize;
        let to = from + bytes_to_read;

        self.inner.buf[from..to].copy_from_slice(&bytes[0..bytes_to_read]);
        self.inner.size = to as u16;

        self.bytes_remaining -= bytes_to_read;
        if self.bytes_remaining == 0 {
            // We successfully read the whole packet.
            let packet = self.inner;
            self.inner.size = 0;
            bytes = &bytes[bytes_to_read..];
            Some((packet, bytes))
        } else {
            None
        }
    }
}
