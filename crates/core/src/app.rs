use core::{fmt::Debug, iter::Cycle, mem::size_of};

use cyberpixie_proto::{DeviceRole, FirmwareInfo};
use embedded_hal::timer::CountDown;

use crate::{
    images::{ImagesRepository, RgbIter},
    leds::{SmartLedsWrite, RGB8},
    proto::{Error, Hertz, Message, Service, SimpleMessage, Transport},
};

const fn core_version() -> [u8; 4] {
    [0, 1, 0, 0]
}

pub struct AppConfig<
    'a,
    Network,
    Timer,
    Images,
    Strip,
    const STRIP_LEN: usize,
    const BUF_LEN: usize,
> where
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

impl<'a, Network, Timer, Images, Strip, const STRIP_LEN: usize, const BUF_LEN: usize>
    AppConfig<'a, Network, Timer, Images, Strip, STRIP_LEN, BUF_LEN>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub fn into_app(self) -> App<'a, Network, Timer, Images, Strip, STRIP_LEN, BUF_LEN> {
        App {
            inner: AppInner {
                device_id: self.device_id,
                timer: self.timer,
                images: self.images,
                strip: self.strip,
                image: None,
            },
            service: Service::new(self.network),
        }
    }
}

pub struct AppInner<'a, Timer, Images, Strip, const STRIP_LEN: usize>
where
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    device_id: [u32; 4],

    timer: Timer,
    images: &'a Images,

    strip: Strip,
    image: Option<(Hertz, Cycle<Images::ImagePixels<'a>>, usize)>,
}

pub struct App<'a, Network, Timer, Images, Strip, const STRIP_LEN: usize, const BUF_LEN: usize>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    inner: AppInner<'a, Timer, Images, Strip, STRIP_LEN>,
    service: Service<Network, BUF_LEN>,
}

impl<'a, Network, Timer, Images, Strip, const STRIP_LEN: usize, const BUF_LEN: usize>
    App<'a, Network, Timer, Images, Strip, STRIP_LEN, BUF_LEN>
where
    Network: Transport,
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
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
        // let mut response = None;
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

        poll_condition!(self.inner.timer.wait(), _, {
            let (rate, line) = self.inner.next_line();
            self.inner
                .strip
                .write(core::array::IntoIter::new(line))
                .expect("Unable to show the next strip line");

            self.inner.timer.start(rate);
        })
        .expect("Unable to write a next strip line");
    }
}

impl<'a, Timer, Images, Strip, const STRIP_LEN: usize> AppInner<'a, Timer, Images, Strip, STRIP_LEN>
where
    Timer: CountDown<Time = Hertz>,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,

    Images::Error: Debug,
{
    fn handle_message<A, I>(&mut self, address: A, msg: Message<I>) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match msg {
            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: STRIP_LEN as u16,
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

        if strip_len != STRIP_LEN {
            return Error::StripLengthMismatch.into();
        }

        let line_len_in_bytes = STRIP_LEN;
        if pixels_len % line_len_in_bytes != 0 {
            return Error::ImageLengthMismatch.into();
        }

        // if pixels_len > buf_len {
        //     return Error::ImageTooBig.into();
        // }

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

    fn next_line(&mut self) -> (Hertz, [RGB8; STRIP_LEN]) {
        let mut line = [RGB8::default(); STRIP_LEN];
        if let Some((rate, image, _)) = self.image.as_mut() {
            (0..line.len()).for_each(|i| line[i] = image.next().unwrap());

            (*rate, line)
        } else {
            (Hertz(50), line)
        }
    }
}
