use core::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    iter::{repeat, Cycle},
};

use heapless::Vec;
use no_stdout::uprintln;
use smart_leds::{SmartLedsWrite, RGB8};

use crate::{
    futures::Stream,
    proto::{DeviceRole, Handshake, Hertz, Service, Transport},
    time::{AsyncCountDown, AsyncTimer},
    AppConfig, HwEvent, Storage,
};

use self::network_task::SecondaryCommand;

mod hw_events_task;
mod image_task;
mod network_task;

const CORE_VERSION: [u8; 4] = [0, 1, 0, 0];
const IDLE_REFRESH_RATE: Hertz = Hertz(50);
const MAX_STRIP_LED_LEN: usize = 144;

struct DeviceLink<T: Transport> {
    address: T::Address,
    data: Handshake,
}

struct DeviceLinks<T: Transport> {
    host: Option<DeviceLink<T>>,
    main: Option<DeviceLink<T>>,
    secondary: Option<DeviceLink<T>>,
}

impl<T: Transport> DeviceLinks<T> {
    fn add_link(&mut self, link: DeviceLink<T>) {
        match link.data.role {
            DeviceRole::Host => self.host.replace(link),
            DeviceRole::Main => self.main.replace(link),
            DeviceRole::Secondary => self.secondary.replace(link),
        };
    }

    #[allow(dead_code)]
    fn get_link(&self, role: DeviceRole) -> &Option<DeviceLink<T>> {
        match role {
            DeviceRole::Host => &self.host,
            DeviceRole::Main => &self.main,
            DeviceRole::Secondary => &self.secondary,
        }
    }

    #[allow(dead_code)]
    fn contains_link(&self, role: DeviceRole) -> bool {
        self.get_link(role).is_some()
    }

    #[allow(dead_code)]
    fn link_data<'a>(
        link: &'a Option<DeviceLink<T>>,
        address: &T::Address,
    ) -> Option<&'a Handshake> {
        link.as_ref()
            .filter(|x| &x.address == address)
            .map(|x| &x.data)
    }

    #[allow(dead_code)]
    fn address_data<'a>(&'a self, address: &T::Address) -> Option<&'a Handshake> {
        Self::link_data(&self.host, address)?;
        Self::link_data(&self.main, address)?;
        Self::link_data(&self.secondary, address)
    }

    fn remove_if_match(link: &mut Option<DeviceLink<T>>, address: &T::Address) -> Option<()> {
        let matched = link.as_ref().filter(|x| &x.address == address).is_some();
        if matched {
            *link = None;
            None
        } else {
            Some(())
        }
    }

    fn remove_address(&mut self, address: &T::Address) -> Option<()> {
        Self::remove_if_match(&mut self.host, address)?;
        Self::remove_if_match(&mut self.main, address)?;
        Self::remove_if_match(&mut self.secondary, address)
    }

    fn secondary_devices(&self) -> impl Iterator<Item = &DeviceLink<T>> {
        self.secondary.iter()
    }
}

impl<T: Transport> Default for DeviceLinks<T> {
    fn default() -> Self {
        Self {
            host: None,
            main: None,
            secondary: None,
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
    pub network: &'a mut Service<Network>,
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
    secondary_command: Option<SecondaryCommand>,

    links: DeviceLinks<Network>,
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

            uprintln!("Showing {} image", index);
        } else {
            uprintln!("Disabling LED strip");
        }

        let index = index as u16;
        if self.app_config.current_image_index != index {
            self.app_config.current_image_index = index;
            self.storage
                .save_config(&self.app_config)
                .expect("unable to update app config");
        }
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
        self.set_image(self.app_config.current_image_index as usize);
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

        // Store the command for further sending it to the secondary device,
        // if there is an unsent command, it will be discarded.
        self.secondary_command
            .replace(SecondaryCommand::ShowImage { index });
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
            secondary_command: None,
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

    fn read_image(&self, index: usize) -> (Hertz, StorageAccess::ImagePixels<'_>) {
        self.inner.borrow().storage.read_image(index)
    }

    fn clear_images(&self) {
        self.inner.borrow_mut().clear_images()
    }

    fn links(&self) -> Ref<DeviceLinks<Network>> {
        Ref::map(self.inner.borrow(), |inner| &inner.links)
    }

    fn links_mut(&self) -> RefMut<DeviceLinks<Network>> {
        RefMut::map(self.inner.borrow_mut(), |inner| &mut inner.links)
    }

    fn safe_mode(&self) -> bool {
        self.inner.borrow().app_config.safe_mode
    }

    fn command_to_secondary(&self) -> Option<SecondaryCommand> {
        self.inner.borrow_mut().secondary_command.take()
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

        let context = Context::new(self.storage, self.role, app_config, self.device_id);
        futures::future::join3(
            context.run_service_events_task(self.network),
            context.run_show_image_task(&mut self.timer, &mut self.strip),
            context.run_hw_events_task(self.events),
        )
        .await
        .0
    }
}

pub(crate) trait ResultExt {
    fn recover(self, msg: &str);
}

impl<E: Debug> ResultExt for Result<(), E> {
    fn recover(self, msg: &str) {
        if let Err(error) = self {
            uprintln!("An error occurred: \"{}\" {:?}", msg, error)
        }
    }
}
