use core::{fmt::Debug, mem::size_of};

use smart_leds::RGB8;

use crate::{
    proto::{
        Error, FirmwareInfo, Handshake, Hertz, Message, Service, ServiceEvent, SimpleMessage,
        Transport,
    },
    storage::RgbIter,
    Storage,
};

use super::{Context, DeviceLink, CORE_VERSION};

impl<'a, StorageAccess, Network> Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    Network::Error: Debug,
    StorageAccess::Error: Debug,
{
    pub async fn run_service_events_task(&self, service: &mut Service<Network>) -> ! {
        loop {
            self.handle_service_event(service).await;
        }
    }

    async fn handle_service_event(&self, service: &mut Service<Network>) {
        let service_event = service
            .next_event()
            .await
            .expect("unable to get next service event");

        match service_event {
            ServiceEvent::Connected { .. } => {
                // TODO
            }
            ServiceEvent::Disconnected { address } => {
                self.links_mut().remove_address(&address);
            }
            ServiceEvent::Message { address, message } => {
                let response = self.handle_message(address, message);
                service
                    .confirm_message(address)
                    .expect("unable to confirm message");

                if let Some((to, response)) = response {
                    service
                        .send_message(to, response)
                        .expect("Unable to send response");
                }
            }
        }
    }

    fn handle_message<I>(
        &self,
        address: Network::Address,
        message: Message<I>,
    ) -> Option<(Network::Address, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match message {
            Message::Handshake(handshake) => {
                let mut links = self.links_mut();
                if links.contains_link(handshake.role) {
                    None
                } else {
                    links.add_link(DeviceLink {
                        address,
                        data: handshake,
                    });
                    Some(SimpleMessage::Handshake(Handshake {
                        device_id: self.device_id,
                        group_id: Some(1), // TODO
                        role: self.role,
                    }))
                }
            }

            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: self.strip_len() as u16,
                version: CORE_VERSION,
                images_count: self.images_count() as u16,
                device_id: self.device_id,
                // TODO implement composite role logic.
                role: self.role,
            })),

            Message::ClearImages => {
                self.clear_images();
                Some(SimpleMessage::Ok)
            }

            Message::AddImage {
                mut bytes,
                refresh_rate,
                strip_len,
            } => {
                let response = self.handle_add_image(bytes.by_ref(), refresh_rate, strip_len);
                // In order to use the reader further, we must read all of the remaining bytes.
                // Otherwise, the reader will be in an inconsistent state.
                for _ in bytes {}

                Some(response)
            }

            Message::ShowImage { index } => {
                if index > self.images_count() {
                    Some(Error::ImageNotFound.into())
                } else {
                    self.set_image(index);
                    Some(SimpleMessage::Ok)
                }
            }

            _ => None,
        };

        response.map(|msg| (address, msg))
    }

    fn handle_add_image<I>(
        &self,
        bytes: &mut I,
        refresh_rate: Hertz,
        strip_len: usize,
    ) -> SimpleMessage
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        if bytes.len() % size_of::<RGB8>() != 0 {
            return Error::ImageLengthMismatch.into();
        }

        let pixels = RgbIter::new(bytes);
        let pixels_len = pixels.len();

        if strip_len != self.strip_len() {
            return Error::StripLengthMismatch.into();
        }

        let line_len_in_bytes = self.strip_len();
        if pixels_len % line_len_in_bytes != 0 {
            return Error::ImageLengthMismatch.into();
        }

        if self.images_count() >= StorageAccess::MAX_IMAGES_COUNT {
            return Error::ImageRepositoryFull.into();
        }

        let count = self.add_image(refresh_rate, pixels);
        Message::ImageAdded { index: count }
    }
}
