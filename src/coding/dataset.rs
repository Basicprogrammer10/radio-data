//! Commodore Dataset like binary encoder / decoder.

use std::f32::consts::PI;

use bitvec::{order::Lsb0, vec::BitVec, view::BitView};

/// Commodore Dataset like binary encoder.
pub struct BinEncoder {
    data: BitVec<u8, Lsb0>,
    index: usize,
    wave: f32,
}

impl BinEncoder {
    /// Create a new encoder from a slice of bytes.
    pub fn _new(data: &[u8]) -> Self {
        let mut out = BitVec::new();
        data.iter().for_each(|x| out.extend(x.view_bits::<Lsb0>()));

        Self {
            data: out,
            index: 0,
            wave: 0.,
        }
    }

    /// Add data to the encoder.
    pub fn _add_data(&mut self, data: &[u8]) {
        data.iter()
            .for_each(|x| self.data.extend(x.view_bits::<Lsb0>()));
    }
}

impl Iterator for BinEncoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index + 1 >= self.data.len() {
            return None;
        }

        let bit = self.data[self.index];
        self.wave += match bit {
            true => 0.03,
            false => 0.06,
        };

        if self.wave > 2. {
            self.index += 1;
            self.wave = 0.03;
        }

        #[cfg(debug_assertions)]
        if self.index % 1000 == 0 {
            print!(
                "\r{:.2}%",
                self.index as f32 / self.data.len() as f32 * 100.0
            );
        }

        let mut val = -(PI * self.wave).sin();
        if !bit {
            val /= 2.;
        }

        Some(val)
    }
}

/// Commodore Dataset like binary decoder.
pub struct _BinDecoder {
    i: usize,
    start: usize,
    last: Option<f32>,
    pub data: BitVec<u8, Lsb0>,
}

impl _BinDecoder {
    /// Create a new decoder.
    pub fn _new() -> Self {
        Self {
            i: 1,
            start: 0,
            last: None,
            data: BitVec::new(),
        }
    }

    /// Adds a sample to the decoder.
    pub fn _add(&mut self, mut val: f32) {
        val += 0.1;
        if self.last.is_none() {
            self.last = Some(val);
            return;
        }

        if val < 0. && self.last.unwrap() >= 0. {
            if self.i - self.start < 10 {
                return;
            }
            self.data.push(self.i - self.start > 50);
            self.start = self.i;
        }

        self.i += 1;
        self.last = Some(val);

        println!(
            "{}",
            self.data
                .clone()
                .into_vec()
                .iter()
                .map(|x| format!("{x:b}"))
                .collect::<String>()
        );
    }

    /// Gets the decoded data as a byte vec.
    pub fn _done(self) -> Vec<u8> {
        self.data.into_vec()
    }
}
