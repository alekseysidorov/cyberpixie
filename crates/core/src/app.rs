use core::{
    cell::RefCell,
    fmt::Debug,
    iter::{repeat, Cycle},
    mem::size_of,
};

use embedded_hal::timer::CountDown;
use heapless::Vec;
use nb_utils::yield_executor;
use smart_leds::{SmartLedsWrite, RGB8};

use crate::{
    proto::{
        DeviceRole, Error, FirmwareInfo, Hertz, Message, Service, ServiceEvent, SimpleMessage,
        Transport,
    },
    storage::RgbIter,
    time::CountDownEx,
    AppConfig, HwEvent, HwEventSource, Storage,
};

const MAX_STRIP_LED_LEN: usize = 144;
const IDLE_REFRESH_RATE: Hertz = Hertz(50);
const CORE_VERSION: [u8; 4] = [0, 1, 0, 0];

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

struct ContextInner<'a, StorageAccess>
where
    StorageAccess: Storage,
{
    app_config: AppConfig,
    storage: &'a StorageAccess,

    refresh_rate: Hertz,
    image: Option<Cycle<StorageAccess::ImagePixels<'a>>>,
}

impl<'a, StorageAccess> ContextInner<'a, StorageAccess>
where
    StorageAccess: Storage,

    StorageAccess::Error: Debug,
{
    fn strip_len(&self) -> usize {
        self.app_config.strip_len as usize
    }

    fn blank_strip(&mut self) {
        self.refresh_rate = IDLE_REFRESH_RATE;
        self.image = None;
    }

    fn show_line<S>(&mut self, strip: &mut S)
    where
        S: SmartLedsWrite<Color = RGB8>,
    {
        let strip_len = self.strip_len();
        if let Some(image) = self.image.as_mut() {
            let line = image
                .by_ref()
                .take(strip_len)
                .collect::<Vec<RGB8, MAX_STRIP_LED_LEN>>();
            strip.write(line.into_iter()).ok();
        } else {
            let line = repeat(RGB8::default()).take(strip_len);
            strip.write(line).ok();
        }
    }

    fn set_image(&mut self, index: usize) {
        self.blank_strip();
        if index > 0 {
            let (refresh_rate, image) = self.storage.read_image(index - 1);
            self.refresh_rate = refresh_rate;
            self.image.replace(image.cycle());
        }

        self.app_config.current_image_index = index as u16;
        self.storage
            .save_config(&self.app_config)
            .expect("unable to update app config")
    }

    fn add_image<I>(&mut self, refresh_rate: Hertz, bytes: I) -> usize
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
    {
        self.blank_strip();

        let new_count = self
            .storage
            .add_image(bytes, refresh_rate)
            .expect("Unable to save image");
        self.set_image(new_count as usize);
        new_count
    }

    fn clear_images(&mut self) {
        self.blank_strip();

        self.storage
            .clear_images()
            .expect("Unable to clear images repository");
        self.set_image(0);
    }

    fn show_next_image(&mut self) {
        let images_count = self.storage.images_count();
        if images_count == 0 {
            return;
        }

        let index = (self.app_config.current_image_index as usize + 1) % (images_count + 1);
        self.set_image(index);
    }
}

struct Context<'a, StorageAccess>
where
    StorageAccess: Storage,

    StorageAccess::Error: Debug,
{
    inner: RefCell<ContextInner<'a, StorageAccess>>,

    device_id: [u32; 4],
}

impl<'a, StorageAccess> Context<'a, StorageAccess>
where
    StorageAccess: Storage,

    StorageAccess::Error: Debug,
{
    fn new(storage: &'a StorageAccess, app_config: AppConfig, device_id: [u32; 4]) -> Self {
        let mut inner = ContextInner {
            app_config,
            storage,
            refresh_rate: IDLE_REFRESH_RATE,
            image: None,
        };

        if !inner.app_config.safe_mode {
            inner.set_image(inner.app_config.current_image_index as usize)
        };

        Self {
            inner: RefCell::new(inner),
            device_id,
        }
    }

    async fn run_show_image_task<T, S>(&self, timer: &mut T, strip: &mut S) -> !
    where
        T: CountDown<Time = Hertz> + 'static,
        S: SmartLedsWrite<Color = RGB8>,
    {
        let line = repeat(RGB8::default()).take(MAX_STRIP_LED_LEN);
        strip.write(line).ok();

        loop {
            timer.start(self.refresh_rate());
            self.show_line(strip);
            timer.wait_async().await;

            yield_executor().await;
        }
    }

    fn show_line<S>(&self, strip: &mut S)
    where
        S: SmartLedsWrite<Color = RGB8>,
    {
        self.inner.borrow_mut().show_line(strip)
    }

    fn refresh_rate(&self) -> Hertz {
        self.inner.borrow().refresh_rate
    }

    fn strip_len(&self) -> usize {
        self.inner.borrow().strip_len()
    }

    fn images_count(&self) -> usize {
        self.inner.borrow().storage.images_count()
    }

    fn set_image(&self, index: usize) {
        self.inner.borrow_mut().set_image(index)
    }

    fn add_image<I>(&self, refresh_rate: Hertz, bytes: I) -> usize
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
    {
        self.inner.borrow_mut().add_image(refresh_rate, bytes)
    }

    fn clear_images(&self) {
        self.inner.borrow_mut().clear_images()
    }

    fn show_next_image(&self) {
        self.inner.borrow_mut().show_next_image()
    }
}

impl<'a, StorageAccess> Context<'a, StorageAccess>
where
    StorageAccess: Storage,

    StorageAccess::Error: Debug,
{
    async fn run_service_events_task<T>(&self, service: &mut Service<T>) -> !
    where
        T: Transport + Unpin + 'static,
        T::Error: Debug,
    {
        loop {
            self.handle_service_event(service).await;

            yield_executor().await;
        }
    }

    async fn handle_service_event<T>(&self, service: &mut Service<T>)
    where
        T: Transport + 'static,
        T::Error: Debug,
    {
        let service_event = service
            .poll_next_event_async()
            .await
            .expect("unable to get next service event");

        match service_event {
            ServiceEvent::Connected { .. } => {
                // TODO
            }
            ServiceEvent::Disconnected { .. } => {
                // TODO
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

    fn handle_message<A, I>(&self, address: A, message: Message<I>) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match message {
            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: self.strip_len() as u16,
                version: CORE_VERSION,
                images_count: self.images_count() as u16,
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

impl<'a, StorageAccess> Context<'a, StorageAccess>
where
    StorageAccess: Storage,

    StorageAccess::Error: Debug,
{
    async fn run_hw_events_task(&self, hw_events: &mut dyn HwEventSource) -> ! {
        loop {
            let hw_event = nb_utils::poll_nb_future(|| hw_events.next_event())
                .await
                .unwrap();
            self.handle_hardware_event(hw_event);

            yield_executor().await;
        }
    }

    fn handle_hardware_event(&self, event: HwEvent) {
        match event {
            HwEvent::ShowNextImage => self.show_next_image(),
        }
    }
}

impl<'a, Network, Timer, StorageAccess, Strip> App<'a, Network, Timer, StorageAccess, Strip>
where
    Network: Transport + Unpin + 'static,
    Timer: CountDown<Time = Hertz> + 'static,
    StorageAccess: Storage + 'static,
    Strip: SmartLedsWrite<Color = RGB8> + 'static,

    Strip::Error: Debug,
    Network::Error: Debug,
    StorageAccess::Error: Debug,
{
    pub async fn run(mut self) -> ! {
        let app_config = self
            .storage
            .load_config()
            .expect("unable to load storage config");

        let context = Context::new(self.storage, app_config, self.device_id);
        futures::future::join3(
            context.run_service_events_task(&mut Service::new(
                self.network,
                app_config.receiver_buf_capacity,
            )),
            context.run_show_image_task(&mut self.timer, &mut self.strip),
            context.run_hw_events_task(self.events),
        )
        .await
        .0
    }
}
