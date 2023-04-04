//! # Range Test
//!
//! Lets you test the range if your radio system.
//! If it receives the DTMF tones defined in the const below,
//! it will play back a tone.

use parking_lot::Mutex;

use super::Module;
use crate::{coding::dtmf::DtmfDecoder, tone::Tone};

pub struct RangeTest {
    dtmf: Mutex<DtmfDecoder>,
    tone: Mutex<Tone>,
}

impl Default for RangeTest {
    fn default() -> Self {
        Self {
            dtmf: Mutex::new(DtmfDecoder::new(callback)),
            tone: Mutex::new(Tone::new(440.).duration(0)),
        }
    }
}

impl Module for RangeTest {
    fn name(&self) -> &'static str {
        "RangeTest"
    }

    fn input(&self, input: &[f32]) {
        self.dtmf.lock().process(input);
    }

    fn output(&self, output: &mut [f32]) {
        let mut tone = self.tone.lock();
        for i in output {
            *i = tone.next().unwrap_or(0.);
        }
    }
}

fn callback(_chr: char) {
    todo!()
}
