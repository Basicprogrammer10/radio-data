use std::f32::consts::PI;

use crate::SAMPLE_RATE;

pub struct Tone {
    i: f32,
    tone: f32,
    sample_rate: f32,
}

impl Tone {
    pub fn new(tone: f32) -> Self {
        Self {
            i: 0_f32,
            sample_rate: SAMPLE_RATE as f32,
            tone,
        }
    }
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1_f32;
        Some((self.i * self.tone * 2.0 * PI / self.sample_rate).sin())
    }
}
