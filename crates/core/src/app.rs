use core::{fmt::Debug, mem::size_of};

use cyberpixie_proto::FirmwareInfo;

use crate::{
    images::{ImagesRepository, RgbIter},
    leds::{SmartLedsWrite, RGB8},
    proto::{types::Hertz, Error, Message, Service, SimpleMessage},
    time::DeadlineTimer,
};

const CORE_VERSION: u32 = 1;

pub struct AppConfig<Network, Timer, Images, Strip, const STRIP_LEN: usize>
where
    Network: Service,
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub network: Network,
    pub timer: Timer,
    pub images_repository: Images,
    pub strip: Strip,
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

impl<Network, Timer, Images, Strip, const STRIP_LEN: usize>
    AppConfig<Network, Timer, Images, Strip, STRIP_LEN>
where
    Network: Service,
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    pub fn into_app(self, buf: &mut [RGB8]) -> App<Network, Timer, Images, Strip, STRIP_LEN> {
        App {
            inner: AppInner {
                timer: self.timer,
                images_repository: self.images_repository,
                strip_state: StripState {
                    image_index: 0,
                    refresh_rate: Hertz(50),
                    current_line: 0,
                    total_lines_count: 0,
                },
                strip: self.strip,
            },
            network: self.network,
            buf,
        }
    }
}

pub struct AppInner<Timer, Images, Strip, const STRIP_LEN: usize>
where
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    timer: Timer,
    images_repository: Images,

    strip: Strip,
    strip_state: StripState<STRIP_LEN>,
}

struct StripState<const STRIP_LEN: usize> {
    image_index: usize,
    refresh_rate: Hertz,
    current_line: usize,
    total_lines_count: usize,
}

impl<const STRIP_LEN: usize> StripState<STRIP_LEN> {
    fn next_line<'a>(&mut self, src: &'a [RGB8]) -> impl Iterator<Item = RGB8> + 'a {
        if self.total_lines_count == 0 {
            return src[0..0].as_ref().iter().copied();
        }

        let from = self.current_line * STRIP_LEN;
        let to = from + STRIP_LEN;

        self.current_line += 1;
        if self.current_line == self.total_lines_count {
            self.current_line = 0;
        }

        src[from..to].as_ref().iter().copied()
    }
}

pub struct App<'a, Network, Timer, Images, Strip, const STRIP_LEN: usize>
where
    Network: Service,
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    inner: AppInner<Timer, Images, Strip, STRIP_LEN>,
    network: Network,
    buf: &'a mut [RGB8],
}

impl<'a, Network, Timer, Images, Strip, const STRIP_LEN: usize>
    App<'a, Network, Timer, Images, Strip, STRIP_LEN>
where
    Network: Service,
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,

    Strip::Error: Debug,
    Network::Error: Debug,
    Timer::Error: Debug,
    Images::Error: Debug,
{
    pub fn run(mut self) -> ! {
        if self.inner.images_repository.count() > 0 {
            self.inner.load_image(self.buf, 0);
        }

        self.event_loop()
    }

    fn event_loop(&mut self) -> ! {
        loop {
            self.event_loop_step();
        }
    }

    fn event_loop_step(&mut self) {
        let mut response = None;
        poll_condition!(self.network.poll_next_message(), (addr, msg), {
            response = self.inner.handle_message(self.buf, addr, msg);
        })
        .expect("Unable to poll next event");

        if let Some((to, response)) = response.take() {
            self.network
                .send_message(to, response)
                .expect("Unable to send response");
        }

        poll_condition!(self.inner.timer.wait_deadline(), _, {
            let line = self.inner.strip_state.next_line(self.buf);
            self.inner
                .strip
                .write(line)
                .expect("Unable to show the next strip line");

            self.inner
                .timer
                .set_deadline(self.inner.strip_state.refresh_rate);
        })
        .expect("Unable to write a next strip line");
    }
}

impl<Timer, Images, Strip, const STRIP_LEN: usize> AppInner<Timer, Images, Strip, STRIP_LEN>
where
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,

    Images::Error: Debug,
{
    fn handle_message<A, I>(
        &mut self,
        buf: &mut [RGB8],
        address: A,
        msg: Message<I>,
    ) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        let response = match msg {
            Message::GetInfo => Some(SimpleMessage::Info(FirmwareInfo {
                strip_len: STRIP_LEN as u16,
                version: CORE_VERSION,
            })),
            Message::ClearImages => {
                self.clear_images();
                Some(SimpleMessage::Ok)
            }
            Message::AddImage {
                bytes,
                refresh_rate,
                strip_len,
            } => {
                let response = self.handle_add_image(buf, bytes, refresh_rate, strip_len);
                // Reload the current image.
                self.load_image(buf, self.strip_state.image_index);
                Some(response)
            }
            Message::ShowImage { index } => {
                if index >= self.images_repository.count() {
                    Some(Error::ImageNotFound.into())
                } else {
                    self.load_image(buf, index);
                    Some(SimpleMessage::Ok)
                }
            }

            _ => None,
        };

        response.map(|msg| (address, msg))
    }

    fn handle_add_image<I>(
        &mut self,
        buf: &mut [RGB8],
        bytes: I,
        refresh_rate: Hertz,
        strip_len: usize,
    ) -> SimpleMessage
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        if bytes.len() % size_of::<RGB8>() != 0 {
            return Error::ImageLengthMismatch.into();
        }

        // Reuse the image buffer to immediately read a new image from the stream
        // to avoid a buffer overrun.
        let mut pixels = RgbIter::new(bytes);
        let pixels_len = pixels.len();
        (0..buf.len()).for_each(|i| {
            if let Some(pixel) = pixels.next() {
                buf[i] = pixel;
            }
        });

        if strip_len != STRIP_LEN {
            return Error::StripLengthMismatch.into();
        }

        let line_len_in_bytes = STRIP_LEN;
        if pixels_len % line_len_in_bytes != 0 {
            return Error::ImageLengthMismatch.into();
        }

        if pixels_len >= buf.len() {
            return Error::ImageTooBig.into();
        }

        if self.images_repository.count() >= Images::MAX_COUNT {
            return Error::ImageRepositoryFull.into();
        }

        let pixels = buf[0..pixels_len].iter().copied();
        let count = self.save_image(refresh_rate, pixels);

        Message::ImageAdded { index: count - 1 }
    }

    fn load_image(&mut self, buf: &mut [RGB8], index: usize) {
        let (refresh_rate, pixels) = self.images_repository.read_image(index);

        self.strip_state.refresh_rate = refresh_rate;
        self.strip_state.total_lines_count = pixels.len() / STRIP_LEN;
        self.strip_state.image_index = index;

        for (index, pixel) in pixels.enumerate() {
            buf[index] = pixel;
        }
    }

    fn save_image<I>(&mut self, refresh_rate: Hertz, bytes: I) -> usize
    where
        I: Iterator<Item = RGB8> + ExactSizeIterator,
    {
        self.images_repository
            .add_image(bytes, refresh_rate)
            .expect("Unable to save image")
    }

    fn clear_images(&mut self) {
        self.strip_state.total_lines_count = 0;
        self.strip_state.current_line = 0;

        self.images_repository
            .clear()
            .expect("Unable to clear images repository");
    }
}
