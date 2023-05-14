//! DTMF tone based binary encoder and decoder.
//! The decoder is based on the [Goertzel algorithm](https://en.wikipedia.org/wiki/Goertzel_algorithm), and as I am writing this comment, over a month after implementing this, I don't remember how it works.

use std::time::Instant;

use bitvec::{order::Lsb0, vec::BitVec, view::BitView};

use crate::{
    audio::{algorithms::goertzel_mag, tone::Tone},
    misc::SampleRate,
};

const COL: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];
const ROW: [f32; 4] = [697.0, 770.0, 852.0, 941.0];
const VAL: [u8; 16] = *b"123A456B789C*0#D";
const MAGNITUDE_EPSILON: f32 = 0.05;
const DATA_LENGTH: usize = 10;
const VALUE_INVALIDATE: usize = 1000;

/// Decode DTMF tones into binary data.
/// Uses the [Goertzel algorithm](https://en.wikipedia.org/wiki/Goertzel_algorithm).
pub struct DtmfDecoder {
    // == Config ==
    sample_rate: SampleRate,

    // == Internal ==
    data: Vec<u8>,
    last: Option<u8>,
    last_timestamp: Instant,
    callback: Box<dyn FnMut(u8) + Send + Sync + 'static>,
}

/// Encode binary data into DTMF tones.
pub struct DtmfEncoder {
    // == Config ==
    sample_rate: SampleRate,
    time: u32,
    sleep: u32,

    // == Internal ==
    low: Tone,
    high: Tone,
    data: Vec<u8>,
    cooldown: usize,
    i: usize,
}

impl DtmfEncoder {
    // 0-9, a, b, d, c, *, #
    /// Create a new encoder from a slice of bytes.
    pub fn new(data: &[u8], sample_rate: SampleRate) -> Self {
        Self {
            time: sample_rate.output / 2,
            sleep: sample_rate.output / 4,
            sample_rate,

            low: Tone::new(0.0, sample_rate),
            high: Tone::new(0.0, sample_rate),
            data: data.to_vec(),
            cooldown: 0,
            i: 0,
        }
    }
}

impl Iterator for DtmfEncoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return Some(0.0);
        }

        if self.i % self.time as usize == 0 {
            let val = self.data.get(self.i / self.time as usize)?;
            let val = VAL.iter().enumerate().find(|x| x.1 == val).unwrap().0 as u8;
            let col = val % COL.len() as u8;
            let row = val / COL.len() as u8;
            self.low = Tone::new(COL[col as usize], self.sample_rate);
            self.high = Tone::new(ROW[row as usize], self.sample_rate);
            self.cooldown = self.sleep as usize;
        }

        self.i = self.i.wrapping_add(1);
        let out = (self.low.next().unwrap() * 0.5) + (self.high.next().unwrap() * 0.5);
        Some(out)
    }
}

impl DtmfDecoder {
    /// Create a new decoder, the callback will be called when a byte is decoded.
    pub fn new(sample_rate: SampleRate, callback: impl FnMut(u8) + Send + Sync + 'static) -> Self {
        Self {
            sample_rate,
            data: Vec::with_capacity(DATA_LENGTH),
            callback: Box::new(callback),
            last_timestamp: Instant::now(),
            last: None,
        }
    }

    /// Add some samples to the decoder.
    /// Will call the callback if a character is decoded.
    pub fn process(&mut self, data: &[f32]) {
        let freqs = ROW
            .iter()
            .chain(COL.iter())
            .map(|x| goertzel_mag(*x, data, self.sample_rate.input))
            .collect::<Vec<_>>();
        let x = match frequencies_to_dtmf(&freqs) {
            Some(i) => i,
            None => return,
        };

        self.data.push(x);
        while self.data.len() > DATA_LENGTH {
            self.data.remove(0);
        }

        let first = self.data[0];
        if self.data.iter().any(|x| *x != first)
            || (Some(first) == self.last
                && self.last_timestamp.elapsed().as_millis() <= VALUE_INVALIDATE as u128)
        {
            return;
        }

        self.last_timestamp = Instant::now();
        (self.callback)(x);
        self.last = Some(x);
    }
}

/// Converts a slice of frequencies to a DTMF characters from [`VAL`].
pub fn frequencies_to_dtmf(freqs: &[f32]) -> Option<u8> {
    let mut row = freqs[0..4].iter().enumerate().collect::<Vec<_>>();
    let mut col = freqs[4..8].iter().enumerate().collect::<Vec<_>>();

    row.sort_by(|a, b| a.1.total_cmp(b.1));
    col.sort_by(|a, b| a.1.total_cmp(b.1));

    let row_max = row.last().unwrap();
    let col_max = col.last().unwrap();

    if *col_max.1 < MAGNITUDE_EPSILON || *row_max.1 < MAGNITUDE_EPSILON {
        return None;
    }

    Some(VAL[row_max.0 * 4 + col_max.0])
}

/// Converts arbitrary binary data into a list of DTMF characters from [`VAL`] (0-15).
pub fn bin_to_dtmf(data: &[u8]) -> Vec<u8> {
    let bits = data.view_bits::<Lsb0>();

    bits.chunks(4)
        .map(|x| {
            VAL[((x[0] as u8) << 3 | (x[1] as u8) << 2 | (x[2] as u8) << 1 | (x[3]) as u8) as usize]
        })
        .collect::<Vec<_>>()
}

/// Decodes a slice of DTMF characters from [`VAL`] back into binary data.
pub fn dtmf_to_bin(dtmf: &[u8]) -> Vec<u8> {
    let mut bits = BitVec::<u8, Lsb0>::new();

    for i in dtmf {
        let val = VAL.iter().enumerate().find(|x| x.1 == i).unwrap().0;
        bits.extend(val.view_bits::<Lsb0>()[0..4].iter().rev());
    }

    bits.into_vec()
}
