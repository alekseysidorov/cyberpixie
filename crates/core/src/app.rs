use core::{fmt::Debug, mem::size_of};

use cyberpixie_proto::FirmwareInfo;

use crate::{
    images::{ImagesRepository, RgbIter},
    leds::{SmartLedsWrite, RGB8},
    proto::{types::Hertz, Message, Service, SimpleMessage},
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
                    refresh_rate: Hertz(50),
                    current_line: 0,
                    total_lines_count: 0,
                },
                strip: self.strip,
                buf,
            },
            network: self.network,
        }
    }
}

pub struct AppInner<'a, Timer, Images, Strip, const STRIP_LEN: usize>
where
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    timer: Timer,
    images_repository: Images,

    strip: Strip,
    strip_state: StripState<STRIP_LEN>,
    buf: &'a mut [RGB8],
}

struct StripState<const STRIP_LEN: usize> {
    refresh_rate: Hertz,
    current_line: usize,
    total_lines_count: usize,
}

impl<const STRIP_LEN: usize> StripState<STRIP_LEN> {
    fn next_line<'a>(&mut self, src: &'a [RGB8]) -> impl Iterator<Item = RGB8> + 'a {
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
    inner: AppInner<'a, Timer, Images, Strip, STRIP_LEN>,
    network: Network,
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
            self.inner.load_image(0);
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
            response = self.inner.handle_message(addr, msg);
        })
        .expect("Unable to poll next event");

        if let Some((to, response)) = response.take() {
            self.network
                .send_message(to, response)
                .expect("Unable to send response");
        }

        poll_condition!(self.inner.timer.wait_deadline(), _, {
            let line = self.inner.strip_state.next_line(self.inner.buf);
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

impl<'a, Timer, Images, Strip, const STRIP_LEN: usize> AppInner<'a, Timer, Images, Strip, STRIP_LEN>
where
    Timer: DeadlineTimer,
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
                // TODO Implement error codes.
                if strip_len != STRIP_LEN {
                    return Some((address, SimpleMessage::Error(1)));
                }

                if self.images_repository.count() >= Images::MAX_COUNT {
                    return Some((address, SimpleMessage::Error(2)));
                }

                let line_len_in_bytes = STRIP_LEN * size_of::<RGB8>();
                if bytes.len() % line_len_in_bytes != 0 {
                    return Some((address, SimpleMessage::Error(3)));
                }

                let pixels = RgbIter::new(bytes);
                let index = self.save_image(refresh_rate, pixels);
                Some(Message::ImageAdded { index })
            }

            _ => None,
        };

        response.map(|msg| (address, msg))
    }

    fn load_image(&mut self, index: usize) {
        let (refresh_rate, pixels) = self.images_repository.read_image(index);

        self.strip_state.refresh_rate = refresh_rate;
        self.strip_state.total_lines_count = pixels.len() / STRIP_LEN;
        for (index, pixel) in pixels.enumerate() {
            self.buf[index] = pixel;
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
        self.images_repository
            .clear()
            .expect("Unable to clear images repository");
    }
}
