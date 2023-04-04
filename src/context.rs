use crate::{coding::BinEncoder, tone::Tone};

enum State {
    HeadPadding(Tone),
    Transmitting,
}

pub struct Context {
    pub encode: BinEncoder,
    state: State,
    sample_rate: u32,
    i: usize,
}

impl Context {
    pub fn new(encode: BinEncoder, sample_rate: u32) -> Self {
        Self {
            encode,
            sample_rate,
            state: State::HeadPadding(Tone::new(440.0, sample_rate)),
            i: 0,
        }
    }
}

impl Iterator for Context {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.i = self.i.wrapping_add(1);

        if self.i > self.sample_rate as usize {
            self.state = State::Transmitting;
        }

        match &mut self.state {
            State::HeadPadding(tone) => tone.next(),
            State::Transmitting => self.encode.next(),
        }
    }
}
