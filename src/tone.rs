use std::f32::consts::PI;

pub struct Tone {
    i: f32,
    tone: f32,
    sample_rate: f32,
    duration: Option<u32>,
}

impl Tone {
    pub fn new(tone: f32, sample_rate: u32) -> Self {
        Self {
            i: 0_f32,
            sample_rate: sample_rate as f32,
            tone,
            duration: None,
        }
    }

    pub fn duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1_f32;

        match self.duration {
            Some(i) if self.i > i as f32 => return None,
            _ => {}
        }

        Some((self.i * self.tone * 2.0 * PI / self.sample_rate).sin())
    }
}
