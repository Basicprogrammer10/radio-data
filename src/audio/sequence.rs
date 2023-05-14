//! Tone sequencer.

use crate::misc::SampleRate;

use super::tone::Tone;

/// A sequence of tones.
/// Will continue to the next tone when the current one is finished.
pub struct Sequence {
    tones: Vec<Tone>,
    index: usize,
}

impl Sequence {
    /// Create a new empty sequence.
    pub fn new() -> Self {
        Self {
            tones: Vec::new(),
            index: 0,
        }
    }

    /// Add a tone to the sequence.
    pub fn _chain(mut self, tone: Tone) -> Self {
        self.tones.push(tone);
        self
    }

    /// Create a sequence from a string.
    /// The format is as follows:
    /// ```text
    /// Freq;time(s)
    /// 440;1.2
    /// ```
    pub fn from_seq(seq: &str, sample_rate: SampleRate) -> Self {
        let mut tones = Vec::new();

        for i in seq.lines() {
            let (freq, time) = i.split_once(';').unwrap();
            let freq = freq.parse::<f32>().unwrap();
            let time = time.parse::<f32>().unwrap();
            tones.push(
                Tone::new(freq, sample_rate).duration((sample_rate.output as f32 * time) as usize),
            );
        }

        Self { tones, index: 0 }
    }
}

impl Iterator for Sequence {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(i) = self.tones.get_mut(self.index)?.next() {
            return Some(i);
        }

        self.index += 1;
        self.next()
    }
}
