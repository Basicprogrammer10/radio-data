//! # Range Test
//!
//! Lets you test the range if your radio system.
//! If it receives the DTMF tones defined in the const below,
//! it will play back a tone.

const CODE: &[u8] = b"ABCD";

use std::sync::Arc;

use cpal::SupportedStreamConfig;
use parking_lot::Mutex;

use super::Module;
use crate::{coding::dtmf::DtmfDecoder, tone::Tone, SAMPLE_RATE};

pub struct RangeTest {
    dtmf: Mutex<Option<DtmfDecoder>>,
    tone: Mutex<Tone>,
    history: Mutex<Vec<u8>>,
}

impl RangeTest {
    pub fn new() -> Arc<Self> {
        let out = Arc::new(Self {
            dtmf: Mutex::new(None),
            tone: Mutex::new(Tone::new(440.).duration(0)),
            history: Mutex::new(Vec::new()),
        });

        let this = out.clone();
        *out.dtmf.lock() = Some(DtmfDecoder::new(move |x| this.callback(x)));

        out
    }

    fn callback(&self, chr: char) {
        println!("[*] Got code: {chr}");
        let mut history = self.history.lock();
        history.push(chr as u8);

        if history.len() >= CODE.len() && &history[history.len() - CODE.len()..] == CODE {
            println!("GOT CODE");
            *self.tone.lock() = Tone::new(440.).duration(SAMPLE_RATE * 3);
            history.clear();
        }
    }
}

impl Module for RangeTest {
    fn name(&self) -> &'static str {
        "RangeTest"
    }

    fn input(&self, input: &[f32], config: Arc<SupportedStreamConfig>) {
        let work = input
            .iter()
            .enumerate()
            .filter(|x| x.0 % config.channels() as usize == 0)
            .map(|x| *x.1)
            .collect::<Vec<_>>();

        self.dtmf.lock().as_mut().unwrap().process(&work);
    }

    fn output(&self, output: &mut [f32], config: Arc<SupportedStreamConfig>) {
        let mut tone = self.tone.lock();

        let mut last = 0.0;
        for (i, e) in output.iter_mut().enumerate() {
            if i % config.channels() as usize == 0 {
                last = tone.next().unwrap_or(0.);
            }

            *e = last;
        }
    }
}
