use std::sync::Arc;

use parking_lot::Mutex;

use crate::coding::dtmf::{self, DtmfEncoder};

use super::{InitContext, Module};

pub struct DtmfSend {
    ctx: InitContext,
    encode: Mutex<DtmfEncoder>,
}

impl DtmfSend {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let sr = ctx.sample_rate();
        let to_send = ctx.args.get_one::<String>("data").unwrap();
        let to_send = &dtmf::bin_to_dtmf(to_send.as_bytes());

        Arc::new(Self {
            ctx,
            encode: Mutex::new(DtmfEncoder::new(to_send, sr)),
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
                last = self.encode.lock().next().unwrap_or(0.);
            }

            *e = last;
        }
    }
}
