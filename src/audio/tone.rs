use std::f32::consts::PI;

use crate::misc::SampleRate;

#[derive(Clone, Copy, Debug)]
pub struct Tone {
    i: usize,
    tone: f32,
    sample_rate: f32,
    duration: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct SmoothTone {
    inner: Tone,
    duration: usize,
    in_point: usize,
    out_point: usize,
}

impl Tone {
    pub fn new(tone: f32, sample_rate: SampleRate) -> Self {
        Self {
            i: 0,
            sample_rate: sample_rate.output as f32,
            tone,
            duration: None,
        }
    }

    pub fn duration(mut self, duration: u32) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn reset(&mut self) {
        self.i = 0;
        self.duration = None;
    }
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;

        match self.duration {
            Some(i) if self.i > i as usize => return None,
            _ => {}
        }

        Some((self.i as f32 * self.tone * 2.0 * PI / self.sample_rate).sin())
    }
}

impl SmoothTone {
    pub fn new(tone: f32, sample_rate: SampleRate, duration: f32) -> Self {
        let in_out = (tone.recip() * sample_rate.output as f32) as usize;
        Self {
            inner: Tone::new(tone, sample_rate),
            duration: (sample_rate.output as f32 * duration) as usize,
            in_point: in_out,
            out_point: in_out,
        }
    }

    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn duration(mut self, duration: f32) -> Self {
        self.duration = (self.inner.sample_rate * duration) as usize;
        self
    }

    pub fn in_point(mut self, in_point: f32) -> Self {
        self.in_point = (self.inner.sample_rate * in_point) as usize;
        self
    }

    pub fn out_point(mut self, out_point: f32) -> Self {
        self.out_point = (self.inner.sample_rate * out_point) as usize;
        self
    }
}

impl Iterator for SmoothTone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let mut raw = self.inner.next()?;

        let tts = self.inner.i as f32 / self.in_point as f32;
        if tts < 1.0 {
            raw *= tts;
        }

        let out_point = self.duration as f32 - self.out_point as f32;
        let tte = (self.inner.i as f32 - out_point) / (self.duration as f32 - out_point);
        if self.inner.i as f32 > out_point {
            raw *= 1.0 - tte;
        }

        Some(raw)
    }
}
