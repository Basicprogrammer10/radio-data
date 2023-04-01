use std::fmt::format;

use bitvec::{order::Lsb0, vec::BitVec};

pub struct BinDecoder {
    i: usize,
    start: usize,
    last: Option<f32>,
    pub data: BitVec<u8, Lsb0>,
}

impl BinDecoder {
    pub fn new() -> Self {
        Self {
            i: 1,
            start: 0,
            last: None,
            data: BitVec::new(),
        }
    }

    pub fn add(&mut self, mut val: f32) {
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
                .map(|x| format!("{:b}", x))
                .collect::<String>()
        );
    }

    pub fn done(self) -> Vec<u8> {
        self.data.into_vec()
    }
}
