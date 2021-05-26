use core::mem::MaybeUninit;

use crate::{
    message::{read_message, IncomingMessage, Message, SimpleMessage},
    transport::{PacketData, PacketKind, Transport},
    types::{Hertz, MessageHeader},
    FirmwareInfo,
};

macro_rules! wait_for_response {
    ($service:expr, $pattern:pat, $then:expr) => {
        match nb::block!($service.poll_next_message())?.1 {
            $pattern => Ok($then),
            $crate::Message::Error(err) => Err(err),
            _ => Err($crate::Error::UnexpectedResponse.into()),
        }
    };

    ($service:expr, Ok) => {
        wait_for_response!($service, $crate::Message::Ok, ())
    };
}

pub type Response<T> = Result<T, crate::Error>;

#[derive(Debug)]
pub struct Service<T> {
    transport: T,
    receiver_buf_capacity: usize
}

impl<T: Transport> Service<T> {
    pub fn new(transport: T, receiver_buf_capacity: usize) -> Self {
        Self { transport, receiver_buf_capacity }
    }

    pub fn poll_next_message(
        &mut self,
    ) -> nb::Result<(T::Address, IncomingMessage<'_, T>), T::Error> {
        let (address, header) = self.poll_for_message_header()?;

        let msg = read_message(address, header, &mut self.transport)?;
        Ok((address, msg))
    }

    pub fn confirm_message(&mut self, from: T::Address) -> Result<(), T::Error> {
        self.transport.confirm_packet(from)
    }

    pub fn send_message<I>(
        &mut self,
        address: T::Address,
        message: Message<I>,
    ) -> Result<(), T::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let (header, payload) = message.into_header_payload();
        self.send_message_header(address, &header)?;
        nb::block!(self.poll_for_confirmation(address))?;

        if let Some(mut payload) = payload {
            let payload_len = self.receiver_buf_capacity - PacketKind::PACKED_LEN;
            while payload.len() != 0 {
                self.transport
                    .send_packet(payload.by_ref().take(payload_len), address)?;
                nb::block!(self.poll_for_confirmation(address))?;
            }
        }

        Ok(())
    }

    pub fn request_firmware_info(
        &mut self,
        address: T::Address,
    ) -> Result<Response<FirmwareInfo>, T::Error> {
        self.send_message(address, SimpleMessage::GetInfo)?;
        let response = wait_for_response!(self, Message::Info(info), info);
        self.confirm_message(address)?;

        Ok(response)
    }

    pub fn clear_images(&mut self, address: T::Address) -> Result<Response<()>, T::Error> {
        self.send_message(address, SimpleMessage::ClearImages)?;
        let response = wait_for_response!(self, Ok);
        self.confirm_message(address)?;

        Ok(response)
    }

    pub fn add_image<I>(
        &mut self,
        address: T::Address,
        refresh_rate: Hertz,
        strip_len: usize,
        bytes: I,
    ) -> Result<Response<usize>, T::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        self.send_message(
            address,
            Message::AddImage {
                refresh_rate,
                strip_len,
                bytes,
            },
        )?;
        let response = wait_for_response!(self, Message::ImageAdded { index }, index);
        self.confirm_message(address)?;

        Ok(response)
    }

    pub fn show_image(
        &mut self,
        address: T::Address,
        index: usize,
    ) -> Result<Response<()>, T::Error> {
        self.send_message(address, SimpleMessage::ShowImage { index })?;
        let response = wait_for_response!(self, Ok);
        self.confirm_message(address)?;

        Ok(response)
    }

    fn send_message_header(
        &mut self,
        address: T::Address,
        header: &MessageHeader,
    ) -> Result<(), T::Error> {
        let mut buf: [u8; MessageHeader::MAX_LEN] = unsafe {
            let buf = MaybeUninit::uninit();
            buf.assume_init()
        };
        // We assume that the buffer has sufficient size, and the message
        // is always successfully encoded.
        let buf = postcard::to_slice(header, &mut buf).unwrap();
        self.transport.send_packet(buf.iter().copied(), address)
    }

    fn poll_for_confirmation(&mut self, address: T::Address) -> nb::Result<(), T::Error> {
        self.transport.poll_for_confirmation(address)
    }

    fn poll_for_message_header(&mut self) -> nb::Result<(T::Address, MessageHeader), T::Error> {
        let packet = self.transport.poll_next_packet()?;
        let msg = match packet.data {
            PacketData::Payload(bytes) => {
                // TODO: At the MPV stage, we assume that the incoming message is always correct.
                postcard::from_bytes(bytes.as_ref()).unwrap()
            }

            PacketData::Confirmed => unreachable!(),
        };

        Ok((packet.address, msg))
    }
}
