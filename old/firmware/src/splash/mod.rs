pub use wandering_light::WanderingLight;

use smart_leds::RGB8;

mod wandering_light;

trait SplashState<const N: usize> {
    type State;

    fn next_line(&mut self) -> Option<(u32, [RGB8; N])>;

    fn next_state(&self) -> Self::State;
}
