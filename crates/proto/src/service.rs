use crate::{types::Hertz, FirmwareInfo, Message, SimpleMessage};

macro_rules! wait_for_response {
    ($service:expr, $pattern:pat, $then:expr) => {
        if let $pattern = nb::block!($service.poll_next_message())?.1 {
            Some($then)
        } else {
            None
        }
    };

    ($service:expr, Ok) => {
        wait_for_response!($service, Message::Ok, ())
    };
}

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
    ) -> Result<Option<FirmwareInfo>, Self::Error> {
        self.send_message(to, SimpleMessage::GetInfo)?;
        let info = wait_for_response!(self, Message::Info(info), info);
        Ok(info)
    }

    fn clear_images(&mut self, to: Self::Address) -> Result<Option<()>, Self::Error> {
        self.send_message(to, SimpleMessage::ClearImages)?;
        let info = wait_for_response!(self, Ok);
        Ok(info)
    }

    fn add_image<I>(
        &mut self,
        to: Self::Address,
        refresh_rate: Hertz,
        strip_len: usize,
        bytes: I,
    ) -> Result<Option<usize>, Self::Error>
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
        let info = wait_for_response!(self, Message::ImageAdded { index }, index);
        Ok(info)
    }

    fn show_image(&mut self, to: Self::Address, index: usize) -> Result<Option<()>, Self::Error> {
        self.send_message(to, SimpleMessage::ShowImage { index })?;
        let info = wait_for_response!(self, Ok);
        Ok(info)
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
