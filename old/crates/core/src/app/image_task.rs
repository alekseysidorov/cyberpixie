use core::{fmt::Debug, iter::repeat};

use smart_leds::{SmartLedsWrite, RGB8};

use crate::{
    nb_utils::yield_executor,
    proto::{Hertz, Transport},
    time::{AsyncCountDown, AsyncTimer},
    Storage,
};

use super::{Context, MAX_STRIP_LED_LEN};

impl<'a, StorageAccess, Network> Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    StorageAccess::Error: Debug,
{
    pub async fn run_show_image_task<T, S>(&self, timer: &mut AsyncTimer<T>, strip: &mut S) -> !
    where
        T: AsyncCountDown + 'static,
        S: SmartLedsWrite<Color = RGB8>,
    {
        let line = repeat(RGB8::default()).take(MAX_STRIP_LED_LEN);
        strip.write(line).ok();

        loop {
            timer.start(self.refresh_rate());
            self.show_line(strip);
            timer.wait().await;

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
}
