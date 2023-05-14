//! Misc functions that aren't large enough to warrant their own file.

use hashbrown::HashMap;

/// Because dealing with sample rates ended up causing problems with mixing up the input and output sample rates, this struct holds both.
/// Functions that need a sample rate will take this entire struct, and just use the input or output sample rate as needed.
/// Sample rate is the number of samples per second.
#[derive(Debug, Clone, Copy)]
pub struct SampleRate {
    /// The number of samples per second in the **input stream**.
    pub input: u32,
    /// The number of samples per second in the **output stream**.
    pub output: u32,
}

impl SampleRate {
    /// Create a new SampleRate struct with the given input and output sample rates.
    pub fn new(input: u32, output: u32) -> Self {
        Self { input, output }
    }

    /// Create a new SampleRate struct, both the input and output sample rates will `hz`.
    pub fn from_hz(hz: u32) -> Self {
        Self::new(hz, hz)
    }
}

impl From<u32> for SampleRate {
    fn from(hz: u32) -> Self {
        Self::from_hz(hz)
    }
}

pub trait Similarity {
    /// Calculate the similarity between two strings.
    fn similarity(&self, other: &Self) -> f64;
}

impl<T: AsRef<str>> Similarity for T {
    fn similarity(&self, other: &Self) -> f64 {
        similarity(self.as_ref(), other.as_ref())
    }
}

/// Uses the dice coefficient to calculate the similarity between two strings.
pub fn similarity(str1: &str, str2: &str) -> f64 {
    let a = str1.replace(' ', "");
    let b = str2.replace(' ', "");

    // Check some simple cases
    if a == b {
        return 1.0;
    }

    if a.len() < 2 || b.len() < 2 {
        return 0.0;
    }

    let mut first_bigrams = HashMap::<&str, i32>::new();
    for i in 0..a.len() - 1 {
        let bigram = &a[i..i + 2];
        let count = first_bigrams.get(bigram).unwrap_or(&0) + 1;
        first_bigrams.insert(bigram, count);
    }

    let mut intersection_size = 0;
    for i in 0..b.len() - 1 {
        let bigram = &b[i..i + 2];
        let count = *first_bigrams.get(bigram).unwrap_or(&0);

        if count > 0 {
            first_bigrams.insert(bigram, count - 1);
            intersection_size += 1;
        }
    }

    (2.0 * intersection_size as f64) / (str1.len() + str2.len() - 2) as f64
}
