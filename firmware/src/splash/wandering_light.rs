use heapless::Vec;
use smart_leds::RGB8;

use super::SplashState;

const BRIGHTNESS: u8 = 255;
const MAX_TRANSITIONS: usize = 10;

#[derive(Clone, Default)]
pub struct WanderingLight<const N: usize> {
    state: State<N>,
}

impl<const N: usize> WanderingLight<N> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<const N: usize> Iterator for WanderingLight<N> {
    type Item = (u32, [RGB8; N]);

    fn next(&mut self) -> Option<Self::Item> {
        let next_line = self.state.next_line();
        if next_line.is_some() {
            return next_line;
        }

        let mut next_state = self.state.next_state();
        core::mem::swap(&mut self.state, &mut next_state);
        self.state.next_line()
    }
}

#[derive(Clone)]
enum State<const N: usize> {
    Ticker(TickerState<N>),
    ColorTransitions(ColorTransitions<N>),
    Final,
}

impl<const N: usize> Default for State<N> {
    fn default() -> Self {
        Self::Ticker(TickerState::default())
    }
}

impl<const N: usize> SplashState<N> for State<N> {
    type State = State<N>;

    fn next_line(&mut self) -> Option<(u32, [RGB8; N])> {
        match self {
            State::Ticker(out) => out.next_line(),
            State::ColorTransitions(out) => out.next_line(),
            State::Final => None,
        }
    }

    fn next_state(&self) -> Self::State {
        match self {
            State::Ticker(out) => out.next_state(),
            State::ColorTransitions(out) => out.next_state(),
            State::Final => State::Final,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct TickerState<const N: usize> {
    always_on: usize,
    wandering_index: usize,
}

impl<const N: usize> SplashState<N> for TickerState<N> {
    type State = State<N>;

    fn next_line(&mut self) -> Option<(u32, [RGB8; N])> {
        if self.always_on == N {
            return None;
        }

        let green = RGB8 {
            r: 0,
            g: BRIGHTNESS,
            b: 0,
        };

        let mut line = [RGB8::default(); N];
        line[self.wandering_index] = green;

        let always_on_pos = N - self.always_on;
        (always_on_pos..N).for_each(|i| {
            line[i] = green;
        });

        self.wandering_index += 1;
        if self.wandering_index == always_on_pos {
            self.wandering_index = 0;
            self.always_on += 1;
        }

        Some((1, line))
    }

    fn next_state(&self) -> Self::State {
        let red = RGB8 {
            r: BRIGHTNESS,
            g: 0,
            b: 0,
        };
        let green = RGB8 {
            r: 0,
            g: BRIGHTNESS,
            b: 0,
        };
        let blue = RGB8 {
            r: 0,
            g: 0,
            b: BRIGHTNESS,
        };

        let transitions = ColorTransitions::new(
            20,
            [
                green,
                red + blue,
                red,
                red + green,
                green + blue,
                blue,
                red + green + blue,
                RGB8::default(),
            ],
        );

        State::ColorTransitions(transitions)
    }
}

#[derive(Clone)]
struct ColorTransitions<const N: usize> {
    transitions: Vec<ColorTransition<N>, MAX_TRANSITIONS>,
}

impl<const N: usize> ColorTransitions<N> {
    fn new<const M: usize>(ticks: u32, colors: [RGB8; M]) -> Self {
        assert!(colors.len() > 2);

        let mut colors = colors.iter().rev().copied();
        let mut to = colors.next().unwrap();

        let mut transitions = Vec::new();
        for from in colors {
            transitions
                .push(ColorTransition { from, to, ticks })
                .map_err(drop)
                .unwrap();
            to = from;
        }

        Self { transitions }
    }
}

impl<const N: usize> SplashState<N> for ColorTransitions<N> {
    type State = State<N>;

    fn next_line(&mut self) -> Option<(u32, [RGB8; N])> {
        if self.transitions.is_empty() {
            return None;
        }

        let line = self.transitions.last_mut().unwrap().next_line();
        if line.is_some() {
            return line;
        }

        self.transitions.pop();
        self.next_line()
    }

    fn next_state(&self) -> Self::State {
        State::Final
    }
}

#[derive(Clone, Copy)]
struct ColorTransition<const N: usize> {
    from: RGB8,
    to: RGB8,
    ticks: u32,
}

impl<const N: usize> ColorTransition<N> {
    fn next_line(&mut self) -> Option<(u32, [RGB8; N])> {
        if self.from == self.to {
            return None;
        }

        // TODO Use Bresenham's line algorithm.
        fn next_step(from: u8, to: u8) -> u8 {
            match to as i16 - from as i16 {
                i if i > 0 => from + 1,
                i if i < 0 => from - 1,
                _ => from,
            }
        }

        let line = [self.from; N];
        self.from.r = next_step(self.from.r, self.to.r);
        self.from.g = next_step(self.from.g, self.to.g);
        self.from.b = next_step(self.from.b, self.to.b);

        Some((N as u32 * self.ticks, line))
    }
}
