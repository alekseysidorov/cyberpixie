use smart_leds::RGB8;

use super::SplashState;

#[derive(Clone)]
pub struct WanderingLight<const N: usize> {
    brightness: u8,
    state: State<N>,
}

impl<const N: usize> Default for WanderingLight<N> {
    fn default() -> Self {
        Self {
            brightness: 64,
            state: State::default(),
        }
    }
}

impl<const N: usize> WanderingLight<N> {
    pub fn new(brightness: u8) -> Self {
        Self {
            brightness,
            ..Self::default()
        }
    }
}

impl<const N: usize> Iterator for WanderingLight<N> {
    type Item = (u32, [RGB8; N]);

    fn next(&mut self) -> Option<Self::Item> {
        let next_line = self.state.next_line(self.brightness);
        if next_line.is_some() {
            return next_line;
        }

        let mut next_state = self.state.next_state(self.brightness);
        core::mem::swap(&mut self.state, &mut next_state);
        self.state.next_line(self.brightness)
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

    fn next_line(&mut self, brightness: u8) -> Option<(u32, [RGB8; N])> {
        match self {
            State::Ticker(out) => out.next_line(brightness),
            State::ColorTransitions(out) => out.next_line(brightness),
            State::Final => None,
        }
    }

    fn next_state(&self, brightness: u8) -> Self::State {
        match self {
            State::Ticker(out) => out.next_state(brightness),
            State::ColorTransitions(out) => out.next_state(brightness),
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

    fn next_line(&mut self, brightness: u8) -> Option<(u32, [RGB8; N])> {
        if self.always_on == N {
            return None;
        }

        let green = RGB8 {
            r: 0,
            g: brightness,
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

        Some((core::cmp::max(10, self.always_on as u32), line))
    }

    fn next_state(&self, brightness: u8) -> Self::State {
        let red = RGB8 {
            r: brightness,
            g: 0,
            b: 0,
        };
        let green = RGB8 {
            r: 0,
            g: brightness,
            b: 0,
        };
        let blue = RGB8 {
            r: 0,
            g: 0,
            b: brightness,
        };

        let transitions = ColorTransitions::new(
            50,
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
    transitions: Vec<ColorTransition<N>>,
}

impl<const N: usize> ColorTransitions<N> {
    fn new<const M: usize>(ticks: u32, colors: [RGB8; M]) -> Self {
        assert!(colors.len() > 2);

        let mut colors = colors.iter().rev().copied();
        let mut to = colors.next().unwrap();

        let mut transitions = Vec::new();
        for from in colors {
            transitions.push(ColorTransition { from, to, ticks });
            to = from;
        }

        Self { transitions }
    }
}

impl<const N: usize> SplashState<N> for ColorTransitions<N> {
    type State = State<N>;

    fn next_line(&mut self, _brightness: u8) -> Option<(u32, [RGB8; N])> {
        if self.transitions.is_empty() {
            return None;
        }

        let line = self.transitions.last_mut().unwrap().next_line();
        if line.is_some() {
            return line;
        }

        self.transitions.pop();
        self.next_line(_brightness)
    }

    fn next_state(&self, _brightness: u8) -> Self::State {
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

        self.from.r = next_step(self.from.r, self.to.r);
        self.from.g = next_step(self.from.g, self.to.g);
        self.from.b = next_step(self.from.b, self.to.b);
        let line = [self.from; N];

        Some((N as u32 * self.ticks, line))
    }
}
