use std::{f32::consts::PI, time::Instant};

use crate::{SAMPLE_RATE, tone::Tone};

/*
- https://github.com/NHollmann/DTMF-Tool/blob/master/src/utils/dtmf.ts
- https://en.wikipedia.org/wiki/Goertzel_algorithm
*/

const COL: [f32; 4] = [1209.0, 1336.0, 1477.0, 1633.0];
const ROW: [f32; 4] = [697.0, 770.0, 852.0, 941.0];
const VAL: [u8; 16] = *b"123A456B789C*0#D";
const MAGNITUDE_EPSILON: f32 = 0.05;
const DATA_LENGTH: usize = 5;
const VALUE_INVALIDATE: usize = 1000;

pub struct DtmfDecoder {
    data: Vec<u8>,
    last: Option<u8>,
    last_timestamp: Instant,
    callback: Box<dyn FnMut(char) + Send + Sync + 'static>,
}

pub struct DtmfEncoder {
    low: Tone,
    high: Tone
}

impl DtmfDecoder {
    pub fn new(callback: impl FnMut(char) + Send + Sync + 'static) -> Self {
        Self {
            data: Vec::with_capacity(DATA_LENGTH),
            callback: Box::new(callback),
            last_timestamp: Instant::now(),
            last: None,
        }
    }

    pub fn process(&mut self, data: &[f32]) {
        let freqs = ROW
            .iter()
            .chain(COL.iter())
            .map(|x| goertzel_mag(*x, data))
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
        (self.callback)(x as char);
        self.last = Some(x);
    }
}

pub fn goertzel_mag(freq: f32, samples: &[f32]) -> f32 {
    let k = (0.5 + (samples.len() as f32 * freq) / SAMPLE_RATE as f32).floor();
    let omega = (2.0 * PI * k) / samples.len() as f32;
    let sin = omega.sin();
    let cos = omega.cos();
    let coeff = cos * 2.0;

    let mut q0;
    let mut q1 = 0.0;
    let mut q2 = 0.0;

    for i in samples {
        q0 = coeff * q1 - q2 + i;
        q2 = q1;
        q1 = q0;
    }

    let real = q1 - q2 * cos;
    let imag = q2 * sin;

    (real.powi(2) + imag.powi(2)).sqrt()
}

// x(...).unwrap() as char
pub fn frequencies_to_dtmf(freqs: &[f32]) -> Option<u8> {
    struct Tmp {
        index: usize,
        mag: f32,
    }

    let hold = |list: &[f32]| {
        list.iter()
            .enumerate()
            .map(|(index, mag)| Tmp { index, mag: *mag })
            .collect::<Vec<_>>()
    };

    let mut row = hold(&freqs[0..4]);
    let mut col = hold(&freqs[4..8]);

    row.sort_by(|a, b| a.mag.total_cmp(&b.mag));
    col.sort_by(|a, b| a.mag.total_cmp(&b.mag));

    let row_max = row.last().unwrap();
    let col_max = col.last().unwrap();

    if col_max.mag < MAGNITUDE_EPSILON || row_max.mag < MAGNITUDE_EPSILON {
        return None;
    }

    Some(VAL[row_max.index * 4 + col_max.index])
}
