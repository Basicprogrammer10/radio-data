use crate::{coding::BinEncoder, tone::Tone, SAMPLE_RATE};

enum State {
    HeadPadding(Tone),
    Transmitting,
}

pub struct Context {
    encode: BinEncoder,
    state: State,
    i: usize,
}

impl Context {
    const HEAD_TIME: usize = SAMPLE_RATE as usize * 3;

    pub fn new(encode: BinEncoder) -> Self {
        Self {
            encode,
            state: State::HeadPadding(Tone::new(440.0)),
            i: 0,
        }
    }

    pub fn next(&mut self) -> Option<f32> {
        self.i = self.i.wrapping_add(1);

        if self.i > Self::HEAD_TIME {
            self.state = State::Transmitting;
        }

        match &mut self.state {
            State::HeadPadding(tone) => tone.next(),
            State::Transmitting => self.encode.next(),
        }
    }
}
