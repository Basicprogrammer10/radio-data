//! Morse code encoding and decoding of text.

use std::{collections::VecDeque, time::Instant};

use crate::{
    audio::{algorithms::goertzel_mag, tone::SmoothTone},
    misc::SampleRate,
};

const MAGNITUDE_EPSILON: f32 = 0.05;
// TODO: maybe use percentage of dit length instead of absolute value
const DURATION_EPSILON: f32 = 0.2;

/// Encodes text into morse code.
pub struct MorseEncoder {
    sample_rate: SampleRate,
    dit_length: u64,

    tone: SmoothTone,
    data: VecDeque<Morse>,
    state: EncodeState,
}

pub struct MorseDecoder {
    sample_rate: SampleRate,
    dit_length: u64,
    frequency: f32,

    data: Vec<Morse>,
    state: bool,
    last_timestamp: Instant,
    callback: Box<dyn FnMut(&char) + Send + Sync + 'static>,
}

/// The different symbols that can be encoded in morse code.
/// - A Dah is three times the length of a Dit.
/// - A space is the length of a Dit.
/// - A word space is the length of a 7 Dits.
///
/// (usually)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Morse {
    /// The smallest unit of time in morse code
    Dit,
    /// Three times the length of a dit
    Dah,
    /// A space between characters
    Gap,
    /// A space between words
    Space,
}

/// The state of the encoder, either in the middle of sending a symbol, waiting a specified delay between symbols, or idle.
/// Idle means the encoder has no more data to send, and is waiting for more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncodeState {
    /// Currently sending morse encoded data
    Sending(SendState),
    /// Sending a space between words or letters
    Waiting(u64),
    /// All data has been sent, waiting for more
    Idle,
}

/// If the encoder state is Sending, this is the state of the current symbol sending operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SendState {
    data: Morse,
    time: u64,
}

impl MorseEncoder {
    /// Create a new encoder with the specified sample rate, frequency, and dit length.
    /// The other symbol lengths are derived from the dit length.
    pub fn new(sample_rate: SampleRate, frequency: f32, dit_length: u64) -> Self {
        Self {
            sample_rate,
            dit_length,
            tone: SmoothTone::new(frequency, sample_rate, 0.0),
            data: VecDeque::new(),
            state: EncodeState::Idle,
        }
    }

    /// Add text data to the encoder.
    pub fn add_data(&mut self, data: &str) -> anyhow::Result<()> {
        let morse = &Morse::from_str(data)?;
        println!("{}", morse_str(morse));
        self.data.extend(morse);
        if self.state == EncodeState::Idle {
            self.try_advance();
        }

        Ok(())
    }

    /// Check if the encoder is idle, meaning it has no more data to send.
    /// In the `morse send` subcommand this will be used to quit when all data has been sent.
    pub fn is_idle(&self) -> bool {
        self.state == EncodeState::Idle
    }

    /// Tries to advance to the next Morse symbol, returns true if it was able to
    fn try_advance(&mut self) -> bool {
        if let Some(i) = self.data.pop_front() {
            let duration = i.duration(self.dit_length);

            self.tone.reset();
            self.tone = self.tone.duration(duration as f32 / 1000.0);

            self.state = EncodeState::Sending(SendState {
                data: i,
                time: duration * self.sample_rate.output as u64 / 1000,
            });

            return true;
        }

        false
    }
}

impl MorseDecoder {
    pub fn new(
        sample_rate: SampleRate,
        frequency: f32,
        dit_length: u64,
        callback: impl FnMut(&char) + Send + Sync + 'static,
    ) -> Self {
        Self {
            sample_rate,
            frequency,
            dit_length,

            data: Vec::new(),
            state: false,
            last_timestamp: Instant::now(),
            callback: Box::new(callback),
        }
    }

    pub fn process(&mut self, data: &[f32]) {
        let mag = goertzel_mag(self.frequency, data, self.sample_rate.input);
        let val = mag > MAGNITUDE_EPSILON;

        if val != self.state {
            let duration = self.last_timestamp.elapsed().as_secs_f32();
            println!("{} -> {} ({}s)", self.state, val, duration);
            self.last_timestamp = Instant::now();
            self.state = val;

            if !val {
                let morse = match Morse::from_duration(
                    &[Morse::Gap, Morse::Space],
                    duration,
                    self.dit_length,
                ) {
                    Some(i) => i,
                    None => return,
                };

                if morse == Morse::Gap {
                    self.data.push(Morse::Gap);
                    return;
                }

                println!("Got Char: {:?}", &self.data);
                self.data.clear();
                return;
            }

            if let Some(i) =
                Morse::from_duration(&[Morse::Dah, Morse::Dit], duration, self.dit_length)
            {
                self.data.push(i);
            }
        }
    }
}

impl Iterator for MorseEncoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sending = match &mut self.state {
            EncodeState::Idle => return Some(0.0),
            EncodeState::Waiting(0) => {
                if !self.try_advance() {
                    self.state = EncodeState::Idle;
                }
                return Some(0.0);
            }
            EncodeState::Waiting(time) => {
                *time -= 1;
                return Some(0.0);
            }
            EncodeState::Sending(s) => s,
        };

        if sending.time == 0 {
            self.state =
                EncodeState::Waiting(self.dit_length * self.sample_rate.output as u64 / 1000);
            return Some(0.0);
        }

        sending.time -= 1;
        let out = match sending.data {
            Morse::Dit | Morse::Dah => self.tone.next().unwrap(),
            Morse::Gap | Morse::Space => 0.0,
        };

        Some(out)
    }
}

impl Morse {
    /// Converts a string into a vector of morse code symbols.
    /// Will return an error if the string contains invalid characters.
    fn from_str(s: &str) -> anyhow::Result<Vec<Self>> {
        let mut result = Vec::new();
        for c in s.chars() {
            let index = match c.to_ascii_uppercase() {
                e @ 'A'..='Z' => e as u8 - b'A',
                e @ '0'..='9' => e as u8 - b'0' + 26,
                ' ' => 56,
                '.' => 36,
                ',' => 37,
                '?' => 38,
                '\'' => 39,
                '!' => 40,
                '/' => 41,
                '(' => 42,
                ')' => 43,
                '&' => 44,
                ':' => 45,
                ';' => 46,
                '=' => 47,
                '+' => 48,
                '-' => 49,
                '_' => 50,
                '"' => 51,
                '$' => 52,
                '@' => 53,
                '¿' => 54,
                '¡' => 55,
                _ => anyhow::bail!("Invalid character: {}", c),
            };

            result.extend_from_slice(MORSE_ENCODING[index as usize].1);
            result.push(Morse::Gap);
        }

        Ok(result)
    }

    fn char_repr(&self) -> &str {
        match self {
            Self::Dit => ".",
            Self::Dah => "-",
            Self::Gap => "",
            Self::Space => " ",
        }
    }

    fn duration(&self, dit_length: u64) -> u64 {
        match self {
            Self::Dit => dit_length,
            Self::Dah => dit_length * 3,
            Self::Gap => dit_length * 3,
            Self::Space => dit_length * 7,
        }
    }

    fn from_duration(options: &[Self], duration: f32, dit_length: u64) -> Option<Self> {
        for &i in options.iter() {
            let delta = i.duration(dit_length) as f32 / 1000. - duration;
            if delta.abs() < DURATION_EPSILON {
                return Some(i);
            }
        }

        None
    }
}

fn morse_str(bits: &[Morse]) -> String {
    bits.iter().map(|i| i.char_repr()).collect()
}

use Morse::*;
/// Maps characters to their morse code representation
const MORSE_ENCODING: [(char, &[Morse]); 57] = [
    ('A', &[Dit, Dah]),
    ('B', &[Dah, Dit, Dit, Dit]),
    ('C', &[Dah, Dit, Dah, Dit]),
    ('D', &[Dah, Dit, Dit]),
    ('E', &[Dit]),
    ('F', &[Dit, Dit, Dah]),
    ('G', &[Dah, Dah, Dit]),
    ('H', &[Dit, Dit, Dit, Dit]),
    ('I', &[Dit, Dit]),
    ('J', &[Dit, Dah, Dah, Dah]),
    ('K', &[Dah, Dit, Dah]),
    ('L', &[Dit, Dah, Dit]),
    ('M', &[Dah, Dah]),
    ('N', &[Dah, Dit]),
    ('O', &[Dah, Dah, Dah]),
    ('P', &[Dit, Dah, Dah, Dit]),
    ('Q', &[Dah, Dah, Dit, Dah]),
    ('R', &[Dit, Dah, Dit]),
    ('S', &[Dit, Dit, Dit]),
    ('T', &[Dah]),
    ('U', &[Dit, Dit, Dah]),
    ('V', &[Dit, Dit, Dit, Dah]),
    ('W', &[Dit, Dah, Dah]),
    ('X', &[Dah, Dit, Dit]),
    ('Y', &[Dah, Dit, Dah, Dah]),
    ('Z', &[Dah, Dah, Dit]),
    ('0', &[Dah, Dah, Dah, Dah, Dah]),
    ('1', &[Dit, Dah, Dah, Dah, Dah]),
    ('2', &[Dit, Dit, Dah, Dah, Dah]),
    ('3', &[Dit, Dit, Dit, Dah, Dah]),
    ('4', &[Dit, Dit, Dit, Dit, Dah]),
    ('5', &[Dit, Dit, Dit, Dit]),
    ('6', &[Dah, Dit, Dit, Dit, Dit]),
    ('7', &[Dah, Dah, Dit, Dit, Dit]),
    ('8', &[Dah, Dah, Dah, Dit, Dit]),
    ('9', &[Dah, Dah, Dah, Dah]),
    ('.', &[Dit, Dah, Dit, Dah, Dit, Dah]),
    (',', &[Dah, Dah, Dit, Dit, Dah, Dah]),
    ('?', &[Dit, Dit, Dah, Dah, Dit, Dit]),
    ('\'', &[Dit, Dah, Dah, Dah, Dah, Dit]),
    ('!', &[Dah, Dit, Dah, Dit, Dah, Dah]),
    ('/', &[Dah, Dit, Dit, Dah]),
    ('(', &[Dah, Dit, Dah, Dah, Dit]),
    (')', &[Dah, Dit, Dah, Dah, Dit, Dah]),
    ('&', &[Dit, Dah, Dit, Dit, Dit]),
    (':', &[Dah, Dah, Dah, Dit, Dit, Dit]),
    (';', &[Dah, Dit, Dah, Dit, Dah, Dit]),
    ('=', &[Dah, Dit, Dit, Dit]),
    ('+', &[Dit, Dah, Dit, Dah, Dit]),
    ('-', &[Dah, Dit, Dit, Dit, Dit, Dah]),
    ('_', &[Dit, Dit, Dah, Dah, Dit, Dah]),
    ('"', &[Dit, Dah, Dit, Dit, Dah, Dit]),
    ('$', &[Dit, Dit, Dit, Dah, Dit, Dit, Dah]),
    ('@', &[Dit, Dah, Dah, Dit, Dah]),
    ('¿', &[Dit, Dit, Dah, Dit, Dah]),
    ('¡', &[Dah, Dah, Dit, Dit, Dit]),
    (' ', &[Space]),
];
