use endian_codec::{DecodeLE, EncodeLE, PackedSize};

#[derive(PackedSize, EncodeLE, DecodeLE)]
struct PacketBody {
    kind: u8,
    len: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketKind {
    Payload(usize),
    Confirmed,
}

impl PacketKind {
    pub const PACKED_LEN: usize = PacketBody::PACKED_LEN;

    pub fn from_reader(reader: impl Iterator<Item = u8>) -> Self {
        let mut header = [0_u8; PacketKind::PACKED_LEN];
        for (idx, byte) in reader.take(header.len()).enumerate() {
            header[idx] = byte;
        }
        Self::decode(&header)
    }

    pub fn encode(self, buf: &mut [u8]) {
        match self {
            PacketKind::Confirmed => PacketBody { kind: 0, len: 0 }.encode_as_le_bytes(buf),
            PacketKind::Payload(payload) => PacketBody {
                kind: 1,
                len: payload as u16,
            }
            .encode_as_le_bytes(buf),
        }
    }

    pub fn decode(buf: &[u8]) -> Self {
        let body = PacketBody::decode_from_le_bytes(buf);
        match body.kind {
            0 => Self::Confirmed,
            1 => PacketKind::Payload(body.len as usize),
            _ => unreachable!(),
        }
    }

    pub fn to_bytes(self) -> [u8; Self::PACKED_LEN] {
        let mut buf = [0_u8; Self::PACKED_LEN];
        self.encode(&mut buf);
        buf
    }
}
