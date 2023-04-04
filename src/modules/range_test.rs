//! # Range Test
//!
//! Lets you test the range if your radio system.
//! If it receives the DTMF tones defined in the const below,
//! it will play back a tone.

const CODE: &[u8] = b"ABCD";

use std::sync::Arc;

use parking_lot::Mutex;

use super::{InitContext, Module};
use crate::{
    audio::{sequence::Sequence, tone::Tone},
    coding::dtmf::DtmfDecoder,
};

pub struct RangeTest {
    ctx: InitContext,
    dtmf: Mutex<Option<DtmfDecoder>>,
    tone: Mutex<Sequence>,
    history: Mutex<Vec<u8>>,
}

impl RangeTest {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let sr = ctx.sample_rate();
        let out = Arc::new(Self {
            ctx,
            dtmf: Mutex::new(None),
            tone: Mutex::new(Sequence::new(sr)),
            history: Mutex::new(Vec::new()),
        });

        let this = out.clone();
        *out.dtmf.lock() = Some(DtmfDecoder::new(sr, move |x| this.callback(x)));

        out
    }

    fn callback(&self, chr: char) {
        println!("[*] Got code: {chr}");
        let mut history = self.history.lock();
        history.push(chr as u8);

        if history.len() >= CODE.len() && &history[history.len() - CODE.len()..] == CODE {
            println!("GOT CODE");
            let sr = self.ctx.sample_rate();
            *self.tone.lock() = Sequence::from_seq("440;5", sr);
            history.clear();
        }
    }
}

impl Module for RangeTest {
    fn name(&self) -> &'static str {
        "RangeTest"
    }

    fn input(&self, input: &[f32]) {
        let work = input
            .iter()
            .enumerate()
            .filter(|x| x.0 % self.ctx.input.channels() as usize == 0)
            .map(|x| *x.1)
            .collect::<Vec<_>>();

        self.dtmf.lock().as_mut().unwrap().process(&work);
    }

    fn output(&self, output: &mut [f32]) {
        let mut tone = self.tone.lock();

        let mut last = 0.0;
        for (i, e) in output.iter_mut().enumerate() {
            if i % self.ctx.output.channels() as usize == 0 {
                last = tone.next().unwrap_or(0.);
            }

            *e = last;
        }
    }
}
