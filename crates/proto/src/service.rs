use core::mem::MaybeUninit;

use nb_utils::NbResultExt;

use crate::{
    message::{read_message, IncomingMessage, Message, SimpleMessage},
    types::{Hertz, MessageHeader},
    FirmwareInfo, Handshake, PacketKind, Transport, TransportEvent,
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
    receiver_buf_capacity: usize,
}

impl<T: Transport> Service<T> {
    pub fn new(transport: T, receiver_buf_capacity: usize) -> Self {
        Self {
            transport,
            receiver_buf_capacity,
        }
    }

    pub fn poll_next_event(&mut self) -> nb::Result<Event<'_, T>, T::Error> {
        Ok(match self.transport.poll_next_event()? {
            TransportEvent::Connected { address } => Event::Connected { address },
            TransportEvent::Disconnected { address } => Event::Disconnected { address },
            TransportEvent::Packet { address, data } => {
                // TODO: At the MPV stage, we assume that the incoming message is always correct.
                let payload = data.payload().unwrap();
                let header = postcard::from_bytes(payload.as_ref()).unwrap();
                Event::Message {
                    address,
                    message: read_message(address, header, &mut self.transport)?,
                }
            }
        })
    }

    pub fn poll_next_message(
        &mut self,
    ) -> nb::Result<(T::Address, IncomingMessage<'_, T>), T::Error> {
        self.poll_next_event().filter_map(Event::message)
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
        self.wait_for_confirmation(address)?;

        if let Some(mut payload) = payload {
            let payload_len = self.receiver_buf_capacity - PacketKind::PACKED_LEN;
            while payload.len() != 0 {
                self.transport
                    .send_packet(payload.by_ref().take(payload_len), address)?;
                self.wait_for_confirmation(address)?;
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

    pub fn handshake(
        &mut self,
        address: T::Address,
        handshake: Handshake,
    ) -> Result<Response<Handshake>, T::Error> {
        self.send_message(address, SimpleMessage::HandshakeRequest(handshake))?;
        let response = wait_for_response!(self, Message::HandshakeResponse(handshake), handshake);
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

    fn wait_for_confirmation(&mut self, address: T::Address) -> Result<(), T::Error> {
        self.transport.wait_for_confirmation(address)
    }
}

pub enum Event<'a, T>
where
    T: Transport,
{
    Connected {
        address: T::Address,
    },
    Disconnected {
        address: T::Address,
    },
    Message {
        address: T::Address,
        message: IncomingMessage<'a, T>,
    },
}

impl<'a, T> Event<'a, T>
where
    T: Transport,
{
    pub fn address(&self) -> &T::Address {
        match self {
            Event::Connected { address } => address,
            Event::Disconnected { address } => address,
            Event::Message { address, .. } => address,
        }
    }

    pub fn connected(self) -> Option<T::Address> {
        if let Event::Connected { address } = self {
            Some(address)
        } else {
            None
        }
    }

    pub fn disconnected(self) -> Option<T::Address> {
        if let Event::Disconnected { address } = self {
            Some(address)
        } else {
            None
        }
    }

    pub fn message(self) -> Option<(T::Address, IncomingMessage<'a, T>)> {
        if let Event::Message { address, message } = self {
            Some((address, message))
        } else {
            None
        }
    }
}
