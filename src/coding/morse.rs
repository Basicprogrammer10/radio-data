use std::collections::VecDeque;

use crate::{audio::tone::Tone, misc::SampleRate};

pub struct MorseEncoder {
    sample_rate: SampleRate,
    dit_length: u64,

    tone: Tone,
    data: VecDeque<Morse>,
    state: EncodeState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Morse {
    /// The smallest unit of time in morse code
    Dit,
    /// Three times the length of a dit
    Dah,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncodeState {
    /// Currently sending morse encoded data
    Sending(SendState),
    /// Sending a space between words or letters
    Waiting(u64),
    /// All data has been sent, waiting for more
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SendState {
    data: Morse,
    time: u64,
}

impl MorseEncoder {
    pub fn new(sample_rate: SampleRate, frequency: f32, dit_length: u64) -> Self {
        Self {
            sample_rate,
            dit_length,
            tone: Tone::new(frequency, sample_rate),
            data: VecDeque::new(),
            state: EncodeState::Idle,
        }
    }

    pub fn add_data(&mut self, data: &str) -> anyhow::Result<()> {
        self.data.extend(&Morse::from_str(data)?);
        if self.state == EncodeState::Idle {
            self.try_advance();
        }

        Ok(())
    }

    /// Tries to advance to the next Morse symbol, returns true if it was able to
    fn try_advance(&mut self) -> bool {
        if let Some(i) = self.data.pop_front() {
            self.state = EncodeState::Sending(SendState {
                data: i,
                time: i.duration(self.dit_length) * self.sample_rate.output as u64 / 1000,
            });

            return true;
        }

        false
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
                EncodeState::Waiting(self.dit_length * 3 * self.sample_rate.output as u64 / 1000);
            return Some(0.0);
        }

        sending.time -= 1;
        Some(self.tone.next().unwrap())
    }
}

impl Morse {
    fn from_str(s: &str) -> anyhow::Result<Vec<Self>> {
        let mut result = Vec::new();
        for c in s.chars() {
            let index = match c.to_ascii_uppercase() {
                'A'..='Z' => c as u8 - b'A',
                '0'..='9' => c as u8 - b'0' + 26,
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
                ' ' => 54,
                _ => anyhow::bail!("Invalid character: {}", c),
            };

            result.extend_from_slice(MORSE_ENCODING[index as usize].1);
        }

        Ok(result)
    }

    fn duration(&self, dit_length: u64) -> u64 {
        match self {
            Self::Dit => dit_length,
            Self::Dah => dit_length * 3,
        }
    }
}

use Morse::*;
const MORSE_ENCODING: [(char, &[Morse]); 56] = [
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
    ('R', &[Dit, Dah]),
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
];
