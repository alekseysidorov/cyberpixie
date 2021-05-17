use cyberpixie_proto::types::Hertz;
use smart_leds::RGB8;

pub trait ImagesRepository {
    type Error;

    type ImageBytes<'a>: Iterator<Item = RGB8> + ExactSizeIterator;

    fn add_image<I>(&mut self, data: I, refresh_rate: Hertz) -> Result<usize, Self::Error>
    where
        I: Iterator<Item = RGB8>;

    fn read_image(&mut self, index: usize) -> (Hertz, Self::ImageBytes<'_>);

    fn count(&self) -> usize;
}
