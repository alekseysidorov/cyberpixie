use crate::{types::Hertz, Error, FirmwareInfo, Message, SimpleMessage};

macro_rules! wait_for_response {
    ($service:expr, $pattern:pat, $then:expr) => {
        match nb::block!($service.poll_next_message())?.1 {
            $pattern => Ok($then),
            $crate::Message::Error(err) => Err(err),
            _ => Err($crate::Error::UnexpectedResponse.into()),
        }
    };

    ($service:expr, Ok) => {
        wait_for_response!($service, Message::Ok, ())
    };
}

pub type Response<T> = Result<T, Error>;

pub trait Service {
    type Error;

    type Address;
    type BytesReader<'a>: Iterator<Item = u8> + ExactSizeIterator + 'a;

    fn poll_next_event(
        &mut self,
    ) -> nb::Result<ServiceEvent<Self::Address, Self::BytesReader<'_>>, Self::Error>;

    fn send_message<I>(
        &mut self,
        to: Self::Address,
        message: Message<I>,
    ) -> Result<(), Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator;

    // Service trait types relied on the associated types and thus cannot be simplified
    // by the type alias
    #[allow(clippy::type_complexity)]
    fn poll_next_message(
        &mut self,
    ) -> nb::Result<(Self::Address, Message<Self::BytesReader<'_>>), Self::Error> {
        let event = self.poll_next_event()?;
        if let ServiceEvent::Data { address, payload } = event {
            Ok((address, payload))
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    fn request_firmware_info(
        &mut self,
        to: Self::Address,
    ) -> Result<Response<FirmwareInfo>, Self::Error> {
        self.send_message(to, SimpleMessage::GetInfo)?;
        let response = wait_for_response!(self, Message::Info(info), info);
        Ok(response)
    }

    fn clear_images(&mut self, to: Self::Address) -> Result<Response<()>, Self::Error> {
        self.send_message(to, SimpleMessage::ClearImages)?;
        Ok(wait_for_response!(self, Ok))
    }

    fn add_image<I>(
        &mut self,
        to: Self::Address,
        refresh_rate: Hertz,
        strip_len: usize,
        bytes: I,
    ) -> Result<Response<usize>, Self::Error>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        self.send_message(
            to,
            Message::AddImage {
                refresh_rate,
                strip_len,
                bytes,
            },
        )?;
        let response = wait_for_response!(self, Message::ImageAdded { index }, index);
        Ok(response)
    }

    fn show_image(&mut self, to: Self::Address, index: usize) -> Result<Response<()>, Self::Error> {
        self.send_message(to, SimpleMessage::ShowImage { index })?;
        Ok(wait_for_response!(self, Ok))
    }
}

#[derive(Debug)]
pub enum ServiceEvent<A, I>
where
    I: Iterator<Item = u8> + ExactSizeIterator,
{
    Connected { address: A },
    Disconnected { address: A },
    Data { address: A, payload: Message<I> },
}
