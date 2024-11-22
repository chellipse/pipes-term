use once_cell::sync::Lazy;
use rand::prelude::*;
use std::fmt;
use std::io::{self, Write};
use std::ops::{Add, Sub};
use std::thread::sleep;
use std::time::Duration;
use term_size;

static DURATION: Lazy<Duration> = Lazy::new(|| {
    std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(50))
});

#[derive(Copy, Clone, Debug)]
struct WrapU32 {
    n: u32,
    max: u32,
}

impl fmt::Display for WrapU32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.n)
    }
}

impl Add for WrapU32 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            n: (self.n + other.n) % (self.max + 1),
            max: self.max,
        }
    }
}

impl Add<u32> for WrapU32 {
    type Output = Self;

    fn add(self, other: u32) -> Self {
        Self {
            n: (self.n + other) % (self.max + 1),
            max: self.max,
        }
    }
}

impl Sub for WrapU32 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let new_n = if self.n >= other.n {
            self.n - other.n
        } else {
            let underflow = other.n - self.n;
            self.max + 1 - (underflow % (self.max + 1))
        };

        Self {
            n: new_n,
            max: self.max,
        }
    }
}

impl Sub<u32> for WrapU32 {
    type Output = Self;

    fn sub(self, other: u32) -> Self {
        let new_n = if self.n >= other {
            self.n - other
        } else {
            let underflow = other - self.n;
            self.max + 1 - (underflow % (self.max + 1))
        };

        Self {
            n: new_n,
            max: self.max,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    fn from_rng(rng: &mut ThreadRng) -> Self {
        Self {
            r: rng.gen::<u8>(),
            g: rng.gen::<u8>(),
            b: rng.gen::<u8>(),
        }
    }

    fn inc(self) -> Self {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        let mut hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        if hue < 0.0 {
            hue += 360.0;
        }

        let saturation = if max == 0.0 { 0.0 } else { delta / max };
        let value = max;

        hue = (hue + 15.0) % 360.0;

        let c = value * saturation;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = value - c;

        let (r1, g1, b1) = if hue < 60.0 {
            (c, x, 0.0)
        } else if hue < 120.0 {
            (x, c, 0.0)
        } else if hue < 180.0 {
            (0.0, c, x)
        } else if hue < 240.0 {
            (0.0, x, c)
        } else if hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self {
            r: ((r1 + m) * 255.0) as u8,
            g: ((g1 + m) * 255.0) as u8,
            b: ((b1 + m) * 255.0) as u8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Di {
    Left,
    Down,
    Up,
    Right,
}

impl Di {
    // const VERTICAL: char = '│';
    // const HORIZONTAL: char = '─';
    // const TOP_LEFT: char = '┌';
    // const TOP_RIGHT: char = '┐';
    // const BOTTOM_LEFT: char = '└';
    // const BOTTOM_RIGHT: char = '┘';

    const VERTICAL: char = '║';
    const HORIZONTAL: char = '═';
    const TOP_LEFT: char = '╔';
    const TOP_RIGHT: char = '╗';
    const BOTTOM_LEFT: char = '╚';
    const BOTTOM_RIGHT: char = '╝';

    fn from_rng(rng: &mut ThreadRng) -> Self {
        match rng.gen::<u32>() % 4 {
            0 => Self::Left,
            1 => Self::Down,
            2 => Self::Up,
            3 => Self::Right,
            _ => unreachable!(),
        }
    }

    fn to_char(&self) -> char {
        match self {
            Self::Up | Self::Down => Self::VERTICAL,
            Self::Right | Self::Left => Self::HORIZONTAL,
        }
    }

    #[rustfmt::skip]
    fn turn(&self, rng: &mut ThreadRng) -> Self {
        let x = rng.gen::<bool>();
        match self {
            Self::Up => if x {Self::Left} else {Self::Right}
            Self::Down => if x {Self::Left} else {Self::Right}
            Self::Left => if x {Self::Down} else {Self::Up}
            Self::Right => if x {Self::Down} else {Self::Up}
        }
    }

    fn turn_and_to_char(&mut self, rng: &mut ThreadRng) -> char {
        let new = self.turn(rng);
        let c = match (*self, new) {
            (Self::Right, Self::Down) | (Self::Up, Self::Left) => Self::TOP_RIGHT,
            (Self::Down, Self::Right) | (Self::Left, Self::Up) => Self::BOTTOM_LEFT,
            (Self::Down, Self::Left) | (Self::Right, Self::Up) => Self::BOTTOM_RIGHT,
            (Self::Up, Self::Right) | (Self::Left, Self::Down) => Self::TOP_LEFT,
            _ => unreachable!(),
        };
        *self = new;
        c
    }
}

#[derive(Debug)]
struct State {
    col: Color,
    di: Di,
    n: u32,
    x: WrapU32,
    y: WrapU32,
    rng: ThreadRng,
}

impl State {
    fn from_rng(mut rng: ThreadRng) -> Self {
        let (w, h) = term_size::dimensions().unwrap();
        let (w, h) = (w as u32, h as u32);
        let rx = rng.gen::<u32>() % w;
        let ry = rng.gen::<u32>() % h;

        let x = WrapU32 { n: rx, max: w };
        let y = WrapU32 { n: ry, max: h };

        Self {
            di: Di::from_rng(&mut rng),
            col: Color::from_rng(&mut rng),
            n: rng.gen::<u32>() % 20,
            x,
            y,
            rng,
        }
    }

    fn run(&mut self) {
        print!("\x1B[2J");

        loop {
            self.col = self.col.inc();
            match self.di {
                Di::Left => self.x = self.x - 1,
                Di::Right => self.x = self.x + 1,
                Di::Down => self.y = self.y + 1,
                Di::Up => self.y = self.y - 1,
            }

            let character = if self.n == 0 {
                self.n = self.rng.gen::<u32>() % 20;
                self.di.turn_and_to_char(&mut self.rng)
            } else {
                self.n -= 1;
                self.di.to_char()
            };

            let string = format!(
                "\x1B[{};{}H\x1B[38;2;{};{};{}m{}",
                self.y, self.x, self.col.r, self.col.g, self.col.b, character
            );

            print!("{string}");
            let _ = io::stdout().flush();

            sleep(*DURATION);
        }
    }
}

fn main() {
    let mut state = State::from_rng(rand::thread_rng());
    state.run();
}
