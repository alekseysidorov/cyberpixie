use core::{
    fmt::Debug,
    iter::{self, Cycle},
    mem::size_of,
};

use embedded_hal::timer::CountDown;

use crate::{
    leds::{SmartLedsWrite, RGB8},
    proto::{
        DeviceRole, Error, FirmwareInfo, Hertz, Message, NbResultExt, Service, ServiceEvent,
        SimpleMessage, Transport,
    },
    storage::{RgbIter, Storage},
    AppConfig, HwEvent, HwEventSource,
};

const fn core_version() -> [u8; 4] {
    [0, 1, 0, 0]
}

pub struct App<'a, Network, Timer, StorageAccess, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    StorageAccess: Storage,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub network: Network,
    pub timer: Timer,
    pub storage: &'a StorageAccess,
    pub strip: Strip,
    pub device_id: [u32; 4],
    pub events: &'a mut dyn HwEventSource,
}

impl<'a, Network, Timer, StorageAccess, Strip> App<'a, Network, Timer, StorageAccess, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub fn into_event_loop(self) -> EventLoop<'a, Network, Timer, StorageAccess, Strip> {
        let app_config = self
            .storage
            .load_config()
            .map_err(drop)
            .expect("unable to read app config");

        EventLoop {
            inner: EventLoopInner {
                device_id: self.device_id,
                events: self.events,
                storage: self.storage,
                strip: self.strip,
                image: None,
                app_config,
            },
            service: Service::new(self.network, app_config.receiver_buf_capacity as usize),
            timer: self.timer,
        }
    }
}

pub struct EventLoopInner<'a, StorageAccess, Strip>
where
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    device_id: [u32; 4],
    app_config: AppConfig,

    strip: Strip,
    events: &'a mut dyn HwEventSource,
    storage: &'a StorageAccess,

    image: Option<(Hertz, Cycle<StorageAccess::ImagePixels<'a>>)>,
}

pub struct EventLoop<'a, Network, Timer, StorageAccess, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    inner: EventLoopInner<'a, StorageAccess, Strip>,

    service: Service<Network>,
    timer: Timer,
}

impl<'a, Network, Timer, StorageAccess, Strip> EventLoop<'a, Network, Timer, StorageAccess, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,

    Strip::Error: Debug,
    Network::Error: Debug,
    StorageAccess::Error: Debug,
{
    pub fn run(mut self) -> ! {
        if self.inner.storage.images_count() > 0 {
            self.inner.load_image(0);
        } else {
            self.inner.blank_strip();
        }

        loop {
            self.process_events();
        }
    }

    fn process_events(&mut self) {
        if let Some(event) = self.inner.events.next_event() {
            self.inner.handle_hardware_event(event);
        }

        if let Some(event) = self
            .service
            .poll_next_event()
            .expect_ok("unable to poll next event")
        {
            match event {
                ServiceEvent::Connected { .. } => {
                    // TODO
                }

                ServiceEvent::Disconnected { .. } => {
                    // TODO
                }

                ServiceEvent::Message { address, message } => {
                    let response = self.inner.handle_message(address, message);
                    if let Some((to, response)) = response {
                        self.service
                            .send_message(to, response)
                            .expect("Unable to send response");
                    }
                }
            }
        }

        if self.timer.wait().is_ok() {
            let refresh_rate = self.inner.show_line();
            self.timer.start(refresh_rate);
        }
    }
}

impl<'a, StorageAccess, Strip> EventLoopInner<'a, StorageAccess, Strip>
where
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,

    StorageAccess::Error: Debug,
{
    fn strip_len(&self) -> usize {
        self.app_config.strip_len as usize
    }

    fn handle_hardware_event(&mut self, event: HwEvent) {
        match event {
            HwEvent::ShowNextImage => {
                if self.storage.images_count() == 0 {
                    return;
                }

                let index = (self.app_config.current_image_index as usize + 1)
                    % self.storage.images_count();
                self.load_image(index);
            }
        }
    }

    fn handle_message<A, I>(&mut self, address: A, msg: Message<I>) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match msg {
            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: self.app_config.strip_len,
                version: core_version(),
                images_count: self.storage.images_count() as u16,
                device_id: self.device_id,
                // TODO implement composite role logic.
                role: DeviceRole::Single,
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
                if index >= self.storage.images_count() {
                    Some(Error::ImageNotFound.into())
                } else {
                    self.load_image(index);
                    Some(SimpleMessage::Ok)
                }
            }

            _ => None,
        };

        response.map(|msg| (address, msg))
    }

    fn handle_add_image<I>(
        &mut self,
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

        if self.storage.images_count() >= StorageAccess::MAX_IMAGES_COUNT {
            return Error::ImageRepositoryFull.into();
        }

        let count = self.save_image(refresh_rate, pixels);
        Message::ImageAdded { index: count - 1 }
    }

    fn load_image(&mut self, index: usize) {
        self.blank_strip();
        let (rate, image) = self.storage.read_image(index);
        self.image.replace((rate, image.cycle()));

        self.app_config.current_image_index = index as u16;
        self.storage
            .save_config(&self.app_config)
            .expect("unable to update app config")
    }

    fn save_image<I>(&mut self, refresh_rate: Hertz, bytes: I) -> usize
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
    {
        self.blank_strip();
        let new_count = self
            .storage
            .add_image(bytes, refresh_rate)
            .expect("Unable to save image");
        self.load_image(self.app_config.current_image_index as usize);
        new_count
    }

    fn blank_strip(&mut self) {
        self.image.take();
    }

    fn clear_images(&mut self) {
        self.blank_strip();
        self.storage
            .clear_images()
            .expect("Unable to clear images repository");
    }

    fn show_line(&mut self) -> Hertz {
        let strip_len = self.strip_len();

        if let Some((rate, image)) = self.image.as_mut() {
            self.strip.write(image.by_ref().take(strip_len)).ok();
            *rate
        } else {
            self.strip
                .write(iter::repeat(RGB8::default()).take(strip_len))
                .ok();
            Hertz(50)
        }
    }
}
