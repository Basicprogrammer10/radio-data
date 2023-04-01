use std::f32::consts::PI;

use bitvec::{order::Lsb0, vec::BitVec, view::BitView};

pub struct BinEncoder {
    data: BitVec<u8, Lsb0>,
    index: usize,
    wave: f32,
}

impl BinEncoder {
    pub fn new(data: &[u8]) -> Self {
        let mut out = BitVec::new();
        data.iter().for_each(|x| out.extend(x.view_bits::<Lsb0>()));

        Self {
            data: out,
            index: 0,
            wave: 0.,
        }
    }

    pub fn add_data(&mut self, data: &[u8]) {
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

        let mut val = -(PI * self.wave).sin();
        if !bit {
            val /= 2.;
        }

        Some(val)
    }
}
