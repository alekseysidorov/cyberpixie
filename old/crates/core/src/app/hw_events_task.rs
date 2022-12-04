use core::fmt::Debug;

use crate::{
    futures::{Stream, StreamExt},
    proto::Transport,
    HwEvent, Storage,
};

use super::Context;

impl<'a, StorageAccess, Network> Context<'a, StorageAccess, Network>
where
    StorageAccess: Storage,
    Network: Transport,

    StorageAccess::Error: Debug,
{
    pub async fn run_hw_events_task(
        &self,
        hw_events: &mut (dyn Stream<Item = HwEvent> + Unpin),
    ) -> ! {
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

    fn show_next_image(&self) {
        self.inner.borrow_mut().show_next_image()
    }
}
