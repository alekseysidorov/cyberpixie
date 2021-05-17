use core::fmt::Debug;

use crate::{
    leds::{SmartLedsWrite, RGB8},
    images::ImagesRepository,
    proto::{types::Hertz, Message, Service, SimpleMessage},
    time::DeadlineTimer,
};

pub struct AppConfig<Network, Timer, Images, Strip, const STRIP_LEN: usize>
where
    Network: Service,
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    network: Network,
    timer: Timer,
    images_repository: Images,
    strip: Strip,
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
        if self.current_line == self.total_lines_count {
            self.current_line = 0;
        }

        let from = self.current_line * STRIP_LEN;
        let to = from + STRIP_LEN;
        self.current_line += 1;

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
{
    pub fn run(self) {
        todo!()
    }

    pub fn event_loop(&mut self) -> ! {
        loop {
            self.event_loop_step();

            // poll_condition!(self.inner.network.poll_next_message(), |(addr, msg)| {
            //     self.handle_message(addr, msg);
            // })
            // .expect("Unable to poll next event");

            // poll_condition!(self.inner.timer.wait_deadline(), |_| {
            //     self.inner
            //         .strip
            //         .write(self.strip_state.next_line(self.buf))
            //         .unwrap();
            //     self.inner.timer.deadline(self.strip_state.refresh_rate);
            // })
            // .expect("Unable to write a next strip line");
        }
    }

    pub fn event_loop_step(&mut self) {
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
            self.inner
                .strip
                .write(self.inner.strip_state.next_line(self.inner.buf))
                .unwrap();
            self.inner
                .timer
                .deadline(self.inner.strip_state.refresh_rate);
        })
        .expect("Unable to write a next strip line");
    }
}

impl<'a, Timer, Images, Strip, const STRIP_LEN: usize> AppInner<'a, Timer, Images, Strip, STRIP_LEN>
where
    Timer: DeadlineTimer,
    Images: ImagesRepository,
    Strip: SmartLedsWrite<Color = RGB8>,
{
    fn handle_message<A, I>(&mut self, address: A, msg: Message<I>) -> Option<(A, SimpleMessage)>
    where
        I: Iterator<Item = u8> + ExactSizeIterator,
    {
        None
    }
}
