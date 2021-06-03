use void::Void;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HwEvent {
    ShowNextImage,
}

pub trait HwEventSource {
    fn next_event(&mut self) -> nb::Result<HwEvent, Void>;
}

impl HwEventSource for () {
    fn next_event(&mut self) -> nb::Result<HwEvent, Void> {
        Err(nb::Error::WouldBlock)
    }
}
