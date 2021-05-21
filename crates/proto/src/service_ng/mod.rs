pub use message::{IncomingMessage, Message, SimpleMessage};

use core::mem::MaybeUninit;
use heapless::Vec;

use crate::{FirmwareInfo, transport::{PacketData, Transport}, types::{Hertz, MessageHeader}};

mod message;

macro_rules! wait_for_response {
    ($service:expr, $pattern:pat, $then:expr) => {
        match nb::block!($service.poll_next_message())?.1 {
            $pattern => Ok($then),
            $crate::service_ng::Message::Error(err) => Err(err),
            _ => Err($crate::Error::UnexpectedResponse.into()),
        }
    };

    ($service:expr, Ok) => {
        wait_for_response!($service, Message::Ok, ())
    };
}

pub type Response<T> = Result<T, crate::Error>;

#[derive(Debug)]
pub struct Service<T, const BUF_LEN: usize> {
    transport: T,
}

impl<T: Transport, const BUF_LEN: usize> Service<T, BUF_LEN> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn poll_next_message(
        &mut self,
    ) -> nb::Result<(T::Address, IncomingMessage<'_, T>), T::Error> {
        let (address, header) = self.poll_for_message_header()?;

        let msg = message::read_message(address, header, &mut self.transport)?;
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
            while payload.len() != 0 {
                let mut buf: Vec<u8, BUF_LEN> = Vec::new();
                buf.extend(payload.by_ref().take(BUF_LEN));

                self.transport.send_packet(buf, address)?;
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

    pub fn show_image(&mut self, address: T::Address, index: usize) -> Result<Response<()>, T::Error> {
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
        let mut buf: [u8; BUF_LEN] = unsafe {
            let buf = MaybeUninit::uninit();
            buf.assume_init()
        };
        // We assume that the buffer has sufficient size, and the message
        // is always successfully encoded.
        let buf = postcard::to_slice(header, &mut buf).unwrap();
        self.transport.send_packet(buf, address)
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

            PacketData::Received => unreachable!(),
        };

        Ok((packet.address, msg))
    }
}
