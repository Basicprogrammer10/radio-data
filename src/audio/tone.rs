//! A static frequency tone generator.
//! This is the basis for most audio in this application.

use std::f32::consts::PI;

use crate::misc::SampleRate;

/// A static frequency tone generator.
#[derive(Clone, Copy, Debug)]
pub struct Tone {
    /// The index of the current sample.
    i: usize,
    /// The frequency of the tone (Hz).
    tone: f32,
    /// The output device's sample rate.
    sample_rate: f32,
    /// An optional duration for the tone in samples.
    /// Will just cut off the tone when the duration is reached.
    duration: Option<usize>,
}

/// An extension of the Tone struct that ramps the volume up and down at the start and end of the tone to prevent popping.
/// The volume is ramped up/down linearly.
#[derive(Clone, Copy, Debug)]
pub struct SmoothTone {
    /// The inner tone generator.
    inner: Tone,
    /// The duration of the tone in samples.
    duration: usize,
    /// How many samples to ramp up the volume for.
    in_point: usize,
    /// How many samples to ramp down the volume for.
    out_point: usize,
}

impl Tone {
    /// Create a new tone with the given frequency and sample rate.
    pub fn new(tone: f32, sample_rate: SampleRate) -> Self {
        Self {
            i: 0,
            sample_rate: sample_rate.output as f32,
            tone,
            duration: None,
        }
    }

    /// Sets the duration of the tone in samples.
    pub fn duration(mut self, duration: usize) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Resets the tone to the beginning.
    /// Useful if you have passed the duration and want to play the tone again.
    pub fn reset(&mut self) {
        self.i = 0;
    }
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;

        match self.duration {
            Some(i) if self.i > i => return None,
            _ => {}
        }

        Some((self.i as f32 * self.tone * 2.0 * PI / self.sample_rate).sin())
    }
}

impl SmoothTone {
    /// Create a new smooth tone with the given frequency, sample rate and duration (in seconds).
    pub fn new(tone: f32, sample_rate: SampleRate, duration: f32) -> Self {
        let in_out = (tone.recip() * sample_rate.output as f32) as usize;
        Self {
            inner: Tone::new(tone, sample_rate),
            duration: (sample_rate.output as f32 * duration) as usize,
            in_point: in_out,
            out_point: in_out,
        }
    }

    /// Reset the inner tone to the beginning.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Sets the duration of the tone in seconds.
    pub fn duration(mut self, duration: f32) -> Self {
        self.duration = (self.inner.sample_rate * duration) as usize;
        self
    }

    /// Sets the point at which the volume ramp up will be complete, in seconds.
    pub fn in_point(mut self, in_point: f32) -> Self {
        self.in_point = (self.inner.sample_rate * in_point) as usize;
        self
    }

    /// Sets the point at which the volume ramp down will begin, in seconds.
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
