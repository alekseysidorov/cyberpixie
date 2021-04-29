pub const MAX_PACKET_LEN: usize = 512;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Packet {
    pub(crate) size: u16,
    pub(crate) buf: [u8; MAX_PACKET_LEN],
}

impl Packet {
    const HEADER_LEN: usize = 2;

    pub const PAYLOAD_LEN: usize = MAX_PACKET_LEN - Self::HEADER_LEN;

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
        assert!(buf.len() > size + Self::HEADER_LEN);

        buf[0..Self::HEADER_LEN].copy_from_slice(&self.size.to_le_bytes());
        buf[Self::HEADER_LEN..Self::HEADER_LEN + size].copy_from_slice(self.as_ref());
        size + Self::HEADER_LEN
    }

    pub fn payload(&self) -> &[u8] {
        self.buf[0..self.size as usize].as_ref()
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
    size_buf: [u8; Packet::HEADER_LEN],
    size_pos: usize,
}

impl Default for PacketReader {
    fn default() -> Self {
        Self {
            bytes_remaining: 0,
            inner: Packet {
                size: 0,
                buf: [0_u8; MAX_PACKET_LEN],
            },
            size_buf: [0_u8; Packet::HEADER_LEN],
            size_pos: 0,
        }
    }
}

impl PacketReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_byte(&mut self, byte: u8) -> Option<&Packet> {
        let buf = [byte];
        self.add_bytes(&buf).map(|x| x.0)
    }

    // TODO Rewrite packet reader internals to returning a Packet reference instead of copying.
    pub fn add_bytes<'a>(&mut self, mut bytes: &'a [u8]) -> Option<(&Packet, &'a [u8])> {
        if bytes.is_empty() {
            return None;
        }

        // In this case, we have to read a packet size.
        if self.bytes_remaining == 0 {
            match (self.size_pos, bytes.len()) {
                // We have enough bytes to read a packet length immediately.
                (0, len) if len >= Packet::HEADER_LEN => {
                    self.size_buf[0..Packet::HEADER_LEN]
                        .copy_from_slice(&bytes[0..Packet::HEADER_LEN]);
                    bytes = &bytes[Packet::HEADER_LEN..];
                }
                // We have not enough bytes to read length right now.
                (0, len) if len == 1 => {
                    self.size_buf[0] = bytes[0];
                    self.size_pos += 1;
                    return None;
                }
                // We can read the remaining length byte.
                _ => {
                    self.size_buf[1] = bytes[0];
                    self.size_pos += 1;
                    bytes = &bytes[1..];
                }
            }

            self.size_pos = 0;
            self.bytes_remaining = u16::from_le_bytes(self.size_buf) as usize;
            self.inner.size = 0;
        }

        // Extend the packet's buffer by the incoming data.
        let bytes_to_read = core::cmp::min(self.bytes_remaining, bytes.len());
        let from = self.inner.size as usize;
        let to = from + bytes_to_read;

        self.inner.buf[from..to].copy_from_slice(&bytes[0..bytes_to_read]);
        self.inner.size = to as u16;

        self.bytes_remaining -= bytes_to_read;
        if self.bytes_remaining == 0 {
            // We successfully read the whole packet.
            bytes = &bytes[bytes_to_read..];
            Some((&self.inner, bytes))
        } else {
            None
        }
    }
}
