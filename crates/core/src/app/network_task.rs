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

#[derive(Debug)]
enum SlaveCommand {
    ShowImage { index: usize },
    AddImage { index: usize },
    ClearImages,
}

#[derive(Default)]
struct MessageResponse {
    response: Option<SimpleMessage>,
    slave_cmd: Option<SlaveCommand>,
}

impl MessageResponse {
    fn empty() -> Self {
        Self::default()
    }

    fn msg(&mut self, msg: SimpleMessage) -> &mut Self {
        self.response = Some(msg);
        self
    }

    fn cmd(&mut self, cmd: SlaveCommand) -> &mut Self {
        self.slave_cmd = Some(cmd);
        self
    }
}

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
            ServiceEvent::Connected { .. } => {}

            ServiceEvent::Disconnected { address } => {
                self.links_mut().remove_address(&address);
            }

            ServiceEvent::Message { address, message } => {
                let output = self.handle_message(address, message);
                service
                    .confirm_message(address)
                    .expect("unable to confirm message");

                if let Some(cmd) = output.slave_cmd {
                    self.send_command_to_slave(service, cmd)
                        .expect("unable to send command to slave device");
                }
                if let Some(msg) = output.response {
                    service
                        .send_message(address, msg)
                        .expect("Unable to send response");
                }
            }
        }
    }

    fn handle_message<I>(&self, address: Network::Address, message: Message<I>) -> MessageResponse
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let mut response = MessageResponse::empty();
        match message {
            Message::HandshakeRequest(handshake) => {
                let mut links = self.links_mut();
                if !links.contains_link(handshake.role) {
                    links.add_link(DeviceLink {
                        address,
                        data: handshake,
                    });

                    response.msg(SimpleMessage::HandshakeResponse(Handshake {
                        device_id: self.device_id,
                        group_id: Some(1), // TODO
                        role: self.role,
                    }));
                }
            }

            Message::GetInfo => {
                response.msg(SimpleMessage::Info(FirmwareInfo {
                    strip_len: self.strip_len() as u16,
                    version: CORE_VERSION,
                    images_count: self.images_count() as u16,
                    device_id: self.device_id,
                    role: self.role,
                }));
            }

            Message::ClearImages => {
                self.clear_images();
                response
                    .cmd(SlaveCommand::ClearImages)
                    .msg(SimpleMessage::Ok);
            }

            Message::AddImage {
                mut bytes,
                refresh_rate,
                strip_len,
            } => {
                let msg = self.handle_add_image(bytes.by_ref(), refresh_rate, strip_len);
                // In order to use the reader further, we must read all of the remaining bytes.
                // Otherwise, the reader will be in an inconsistent state.
                assert_eq!(bytes.len(), 0);

                if let Message::ImageAdded { index } = &msg {
                    response.cmd(SlaveCommand::AddImage { index: *index - 1 });
                }
                response.msg(msg);
            }

            Message::ShowImage { index } => {
                if index > self.images_count() {
                    response.msg(Error::ImageNotFound.into());
                } else {
                    self.set_image(index);

                    response
                        .msg(SimpleMessage::Ok)
                        .cmd(SlaveCommand::ShowImage { index });
                }
            }

            _ => {}
        };

        response
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

    fn send_command_to_slave(
        &self,
        service: &mut Service<Network>,
        cmd: SlaveCommand,
    ) -> Result<(), Network::Error> {
        for link in self.links().slave_devices() {
            let address = link.address;
            match cmd {
                SlaveCommand::ShowImage { index } => {
                    service.show_image(address, index)?.ok();
                }

                SlaveCommand::ClearImages if !self.save_mode() => {
                    service.clear_images(address)?.ok();
                }

                SlaveCommand::AddImage { index } if !self.save_mode() => {
                    let strip_len = self.strip_len();
                    let (refresh_rate, pixels) = self.read_image(index);

                    service
                        .add_image(address, refresh_rate, strip_len, rgb8_to_bytes(pixels))?
                        .ok();
                }

                _ => {}
            }
        }

        Ok(())
    }
}

fn rgb8_to_bytes<I>(iter: I) -> Rgb8ToBytesIter<impl Iterator<Item = u8>>
where
    I: Iterator<Item = RGB8> + ExactSizeIterator,
{
    let bytes_remaining = iter.len() * size_of::<RGB8>();

    Rgb8ToBytesIter {
        inner: iter.map(|rgb| [rgb.r, rgb.g, rgb.b]).flatten(),
        bytes_remaining,
    }
}

struct Rgb8ToBytesIter<T: Iterator<Item = u8>> {
    inner: T,
    bytes_remaining: usize,
}

impl<T> Iterator for Rgb8ToBytesIter<T>
where
    T: Iterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.inner.next() {
            self.bytes_remaining -= 1;
            Some(byte)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.bytes_remaining, Some(self.bytes_remaining))
    }
}

impl<T: Iterator<Item = u8>> ExactSizeIterator for Rgb8ToBytesIter<T> {}
