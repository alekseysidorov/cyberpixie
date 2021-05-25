#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HwEvent {
    ShowNextImage,
}

pub trait HwEventSource {
    fn next_event(&mut self) -> Option<HwEvent>;
}

impl HwEventSource for () {
    fn next_event(&mut self) -> Option<HwEvent> {
        None
    }
}
