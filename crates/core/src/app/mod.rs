use core::{
    cell::{RefCell, RefMut},
    fmt::Debug,
    iter::{repeat, Cycle},
};

use futures::StreamExt;
use heapless::Vec;
use smart_leds::{SmartLedsWrite, RGB8};

use crate::{
    futures::Stream,
    proto::{DeviceRole, Handshake, Hertz, Service, Transport},
    time::{AsyncCountDown, AsyncTimer},
    AppConfig, HwEvent, Storage,
};

mod image_task;
mod network_task;

const CORE_VERSION: [u8; 4] = [0, 1, 0, 0];
const IDLE_REFRESH_RATE: Hertz = Hertz(50);
const MAX_STRIP_LED_LEN: usize = 144;

struct DeviceLink<A> {
    address: A,
    data: Handshake,
}

struct DeviceLinks<A> {
    host: Option<DeviceLink<A>>,
    master: Option<DeviceLink<A>>,
    slave: Option<DeviceLink<A>>,
}

impl<A: PartialEq> DeviceLinks<A> {
    fn add_link(&mut self, link: DeviceLink<A>) {
        match link.data.role {
            DeviceRole::Host => self.host.replace(link),
            DeviceRole::Master => self.master.replace(link),
            DeviceRole::Slave => self.slave.replace(link),
        };
    }

    fn get_link(&self, role: DeviceRole) -> &Option<DeviceLink<A>> {
        match role {
            DeviceRole::Host => &self.host,
            DeviceRole::Master => &self.master,
            DeviceRole::Slave => &self.slave,
        }
    }

    fn remove_if_match(link: &mut Option<DeviceLink<A>>, address: &A) -> Option<()> {
        let matched = link.as_ref().filter(|x| &x.address == address).is_some();
        if matched {
            *link = None;
            None
        } else {
            Some(())
        }
    }

    fn remove_address(&mut self, address: &A) -> Option<()> {
        Self::remove_if_match(&mut self.host, address)?;
        Self::remove_if_match(&mut self.master, address)?;
        Self::remove_if_match(&mut self.slave, address)
    }

    fn contains_link(&self, role: DeviceRole) -> bool {
        self.get_link(role).is_some()
    }
}

impl<A> Default for DeviceLinks<A> {
    fn default() -> Self {
        Self {
            host: None,
            master: None,
            slave: None,
        }
    }
}

pub struct App<'a, Network, CountDown, StorageAccess, Strip>
where
    Network: Transport,
    CountDown: AsyncCountDown,
    StorageAccess: Storage,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub role: DeviceRole,
    pub network: Network,
    pub timer: AsyncTimer<CountDown>,
    pub storage: &'a StorageAccess,
    pub strip: Strip,
    pub device_id: [u32; 4],
    pub events: &'a mut (dyn Stream<Item = HwEvent> + Unpin),
}

struct ContextInner<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,
{
    app_config: AppConfig,
    storage: &'a StorageAccess,

    refresh_rate: Hertz,
    image: Option<Cycle<StorageAccess::ImagePixels<'a>>>,

    links: DeviceLinks<Network::Address>,
}

impl<'a, StorageAccess, Network> ContextInner<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

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

struct Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    StorageAccess::Error: Debug,
{
    inner: RefCell<ContextInner<'a, StorageAccess, Network>>,

    role: DeviceRole,
    device_id: [u32; 4],
}

impl<'a, StorageAccess, Network> Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    StorageAccess::Error: Debug,
{
    fn new(
        storage: &'a StorageAccess,
        role: DeviceRole,
        app_config: AppConfig,
        device_id: [u32; 4],
    ) -> Self {
        let mut inner = ContextInner {
            app_config,
            storage,
            refresh_rate: IDLE_REFRESH_RATE,
            image: None,
            links: DeviceLinks::default(),
        };

        if !inner.app_config.safe_mode {
            inner.set_image(inner.app_config.current_image_index as usize)
        };

        Self {
            inner: RefCell::new(inner),
            device_id,
            role,
        }
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

    fn links_mut(&self) -> RefMut<DeviceLinks<Network::Address>> {
        RefMut::map(self.inner.borrow_mut(), |inner| &mut inner.links)
    }
}

impl<'a, StorageAccess, Network> Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    StorageAccess::Error: Debug,
{
    async fn run_hw_events_task(&self, hw_events: &mut (dyn Stream<Item = HwEvent> + Unpin)) -> ! {
        loop {
            let hw_event = hw_events.next().await.unwrap();
            self.handle_hardware_event(hw_event);
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
    Timer: AsyncCountDown + 'static,
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

        let context =
            Context::<_, Network>::new(self.storage, self.role, app_config, self.device_id);
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
