use core::{fmt::Debug, iter::Cycle, mem::size_of};

use cyberpixie_proto::{DeviceRole, FirmwareInfo};
use embedded_hal::timer::CountDown;

use crate::{
    images::{ImagesRepository, RgbIter},
    leds::{SmartLedsWrite, RGB8},
    proto::{Error, Hertz, Message, Service, SimpleMessage, Transport},
    HwEvent, HwEventSource,
};

const fn core_version() -> [u8; 4] {
    [0, 1, 0, 0]
}

pub struct AppConfig<'a, Network, Timer, Images, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub network: Network,
    pub timer: Timer,
    pub images: &'a Images,
    pub strip: Strip,
    pub device_id: [u32; 4],
    pub events: &'a mut dyn HwEventSource,
    pub receiver_buf_capacity: usize,
    pub strip_len: usize,
}

macro_rules! poll_condition {
    ($cond:expr, $value:pat, $then:expr) => {{
        match $cond {
            Ok($value) => {
                $then;
                Ok(())
            }
            Err(nb::Error::WouldBlock) => Ok(()),
            Err(nb::Error::Other(err)) => Err(err),
        }
    }};

    ($cond:expr, $then:expr) => {
        poll_condition!($cond, _, $then)
    };
}

impl<'a, Network, Timer, Images, Strip> AppConfig<'a, Network, Timer, Images, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub fn into_event_loop(self) -> EventLoop<'a, Network, Timer, Images, Strip> {
        EventLoop {
            inner: EventLoopInner {
                device_id: self.device_id,
                strip_len: self.strip_len,
                events: self.events,
                images: self.images,
                strip: self.strip,
                image: None,
            },
            service: Service::new(self.network, self.receiver_buf_capacity),
            timer: self.timer,
        }
    }
}

pub struct EventLoopInner<'a, Images, Strip>
where
    Images: ImagesRepository + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    device_id: [u32; 4],
    strip_len: usize,

    events: &'a mut dyn HwEventSource,

    images: &'a Images,

    strip: Strip,
    image: Option<(Hertz, Cycle<Images::ImagePixels<'a>>, usize)>,
}

pub struct EventLoop<'a, Network, Timer, Images, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    inner: EventLoopInner<'a, Images, Strip>,

    service: Service<Network>,
    timer: Timer,
}

impl<'a, Network, Timer, Images, Strip> EventLoop<'a, Network, Timer, Images, Strip>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,

    Strip::Error: Debug,
    Network::Error: Debug,
    Images::Error: Debug,
{
    pub fn run(mut self) -> ! {
        if self.inner.images.count() > 0 {
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
            self.process_hw_event(event);
        }

        poll_condition!(self.service.poll_next_message(), (addr, msg), {
            let response = self.inner.handle_message(addr, msg);
            self.service
                .confirm_message(addr)
                .expect("Unable to confirm message");

            if let Some((to, response)) = response {
                self.service
                    .send_message(to, response)
                    .expect("Unable to send response");
            }
        })
        .expect("Unable to poll next event");

        poll_condition!(self.timer.wait(), _, {
            let refresh_rate = self.inner.show_line();
            self.timer.start(refresh_rate);
        })
        .expect("Unable to write a next strip line");
    }

    fn process_hw_event(&mut self, event: HwEvent) {
        match event {
            HwEvent::ShowNextImage => {
                if self.inner.images.count() == 0 {
                    return;
                }

                let next_index = self.inner.image.take().map(|x| x.2 + 1).unwrap_or_default()
                    % self.inner.images.count();
                self.inner.load_image(next_index);
            }
        }
    }
}

impl<'a, Images, Strip> EventLoopInner<'a, Images, Strip>
where
    Images: ImagesRepository + 'static,
    Strip: SmartLedsWrite<Color = RGB8>,

    Images::Error: Debug,
{
    fn handle_message<A, I>(&mut self, address: A, msg: Message<I>) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match msg {
            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: self.strip_len as u16,
                version: core_version(),
                images_count: self.images.count() as u16,
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
                if index >= self.images.count() {
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

        if strip_len != self.strip_len {
            return Error::StripLengthMismatch.into();
        }

        let line_len_in_bytes = self.strip_len;
        if pixels_len % line_len_in_bytes != 0 {
            return Error::ImageLengthMismatch.into();
        }

        if self.images.count() >= Images::MAX_COUNT {
            return Error::ImageRepositoryFull.into();
        }

        let count = self.save_image(refresh_rate, pixels);
        Message::ImageAdded { index: count - 1 }
    }

    fn load_image(&mut self, index: usize) {
        let (rate, image) = self.images.read_image(index);
        self.image.replace((rate, image.cycle(), index));
    }

    fn save_image<I>(&mut self, refresh_rate: Hertz, bytes: I) -> usize
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
    {
        let index = self.image.take().map(|x| x.2);
        let new_count = self
            .images
            .add_image(bytes, refresh_rate)
            .expect("Unable to save image");

        if let Some(index) = index {
            self.load_image(index);
        }
        new_count
    }

    fn blank_strip(&mut self) {
        self.image.take();
    }

    fn clear_images(&mut self) {
        self.blank_strip();
        self.images
            .clear()
            .expect("Unable to clear images repository");
    }

    fn show_line(&mut self) -> Hertz {
        if let Some((rate, image, _)) = self.image.as_mut() {
            self.strip.write(image.by_ref().take(self.strip_len)).ok();
            *rate
        } else {
            self.strip
                .write(core::iter::repeat(RGB8::default()).take(self.strip_len))
                .ok();
            Hertz(50)
        }
    }
}
