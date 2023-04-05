use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use parking_lot::Mutex;

use crate::{
    audio::tone::Tone,
    coding::dtmf::{self, DtmfEncoder},
};

use super::{InitContext, Module};

pub struct DtmfSend {
    ctx: InitContext,
    state: Mutex<State>,
    encode: Mutex<DtmfEncoder>,
    i: AtomicUsize,
}

enum State {
    Transmitting,
    Head(Tone),
}

impl DtmfSend {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let sr = ctx.sample_rate();
        let to_send = ctx.args.get_one::<String>("data").unwrap();
        let mut to_send = dtmf::bin_to_dtmf(to_send.as_bytes());
        to_send.insert(0, 'A' as u8);
        to_send.insert(1, '#' as u8);
        to_send.push('#' as u8);
        to_send.push('D' as u8);
        

        println!(
            "[D] {}",
            to_send.iter().map(|x| *x as char).collect::<String>()
        );

        Arc::new(Self {
            ctx,
            i: AtomicUsize::new(0),
            state: Mutex::new(State::Head(Tone::new(440.0, sr))),
            encode: Mutex::new(DtmfEncoder::new(&to_send, sr)),
        })
    }
}

impl Module for DtmfSend {
    fn name(&self) -> &'static str {
        "DtmfSend"
    }

    fn output(&self, output: &mut [f32]) {
        let mut last = 0.0;
        for (i, e) in output.iter_mut().enumerate() {
            if i % self.ctx.output.channels() as usize == 0 {
                if self.i.fetch_add(1, Ordering::Relaxed) > self.ctx.input.sample_rate().0 as usize
                {
                    *self.state.lock() = State::Transmitting;
                }

                let mut enc = self.state.lock();
                last = match &mut *enc {
                    State::Head(i) => i.next(),
                    State::Transmitting => self.encode.lock().next(),
                }
                .unwrap_or(0.);
            }

            *e = last;
        }
    }
}
