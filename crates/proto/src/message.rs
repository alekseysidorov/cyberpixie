use core::iter::Empty;

use crate::{
    transport::Transport,
    types::{AddImage, FirmwareInfo, Hertz, MessageHeader},
    Error,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Message<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    // Requests.
    GetInfo,
    AddImage {
        refresh_rate: Hertz,
        strip_len: usize,
        bytes: I,
    },
    ShowImage {
        index: usize,
    },
    ClearImages,

    // Responses.
    Ok,
    ImageAdded {
        index: usize,
    },
    Info(FirmwareInfo),
    Error(Error),
}

impl<I> Message<I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    pub(super) fn into_header_payload(self) -> (MessageHeader, Option<I>) {
        match self {
            Message::GetInfo => (MessageHeader::GetInfo, None),
            Message::AddImage {
                refresh_rate,
                strip_len,
                bytes,
            } => (
                MessageHeader::AddImage(AddImage {
                    refresh_rate,
                    strip_len: strip_len as u16,
                    bytes_len: bytes.len() as u32,
                }),
                Some(bytes),
            ),
            Message::ClearImages => (MessageHeader::ClearImages, None),
            Message::ShowImage { index } => (MessageHeader::ShowImage(index as u16), None),

            Message::ImageAdded { index } => (MessageHeader::ImageAdded(index as u16), None),
            Message::Ok => (MessageHeader::Ok, None),
            Message::Info(info) => (MessageHeader::Info(info), None),
            Message::Error(err) => (MessageHeader::Error(err.into_code()), None),
        }
    }
}

pub type SimpleMessage = Message<Empty<u8>>;
pub type IncomingMessage<'a, T> = Message<PayloadReader<'a, T>>;

pub(super) fn read_message<T: Transport>(
    address: T::Address,
    header: MessageHeader,
    transport: &mut T,
) -> Result<IncomingMessage<'_, T>, T::Error> {
    let msg = match header {
        MessageHeader::GetInfo => Message::GetInfo,
        MessageHeader::ClearImages => Message::ClearImages,
        MessageHeader::AddImage(img) => Message::AddImage {
            refresh_rate: img.refresh_rate,
            bytes: PayloadReader::new(address, transport, img.bytes_len as usize)?,
            strip_len: img.strip_len as usize,
        },
        MessageHeader::ShowImage(index) => Message::ShowImage {
            index: index as usize,
        },

        MessageHeader::Info(info) => Message::Info(info),
        MessageHeader::Ok => Message::Ok,
        MessageHeader::Error(code) => Message::Error(Error::from_code(code)),
        MessageHeader::ImageAdded(index) => Message::ImageAdded {
            index: index as usize,
        },
    };

    Ok(msg)
}

impl From<Error> for SimpleMessage {
    fn from(err: Error) -> Self {
        SimpleMessage::Error(err)
    }
}

pub struct PayloadReader<'a, T: Transport> {
    address: T::Address,
    transport: &'a mut T,

    bytes_remaining: usize,

    read_pos: usize,
    payload: T::Payload,
}

impl<'a, T: Transport> PayloadReader<'a, T> {
    fn new(
        address: T::Address,
        transport: &'a mut T,
        bytes_remaining: usize,
    ) -> Result<Self, T::Error> {
        transport.confirm_packet(address)?;
        let payload = nb::block!(transport.poll_for_payload(address))?;

        Ok(Self {
            address,
            transport,
            bytes_remaining,

            read_pos: 0,
            payload,
        })
    }
}

impl<'a, T: Transport> Iterator for PayloadReader<'a, T> {
    type Item = u8;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.bytes_remaining, Some(self.bytes_remaining))
    }

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.bytes_remaining == 0 {
                return None;
            }

            let payload_bytes = self.payload.as_ref();
            if self.read_pos != payload_bytes.len() {
                let byte = payload_bytes[self.read_pos];
                self.read_pos += 1;
                self.bytes_remaining -= 1;
                return Some(byte);
            }

            self.transport
                .confirm_packet(self.address)
                // To reduce text section size we should avoid Debug traits usage,
                // we still know the code line where panic occurs.
                .map_err(drop)
                .unwrap();

            self.payload = nb::block!(self.transport.poll_for_payload(self.address))
                .map_err(drop)
                .unwrap();
            self.read_pos = 0;
        }
    }
}

impl<'a, T: Transport> ExactSizeIterator for PayloadReader<'a, T> {}

// FIXME: Rethink data reader to enable this code.
// impl<'a, T: Transport> Drop for PayloadReader<'a, T> {
//     fn drop(&mut self) {
//         // In order to use the reader further, we must read all of the remaining bytes.
//         // Otherwise, the reader will be in an inconsistent state.
//         for _ in self {}
//     }
// }
