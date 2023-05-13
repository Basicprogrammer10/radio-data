use std::sync::Arc;

use parking_lot::Mutex;

use crate::coding::morse::MorseEncoder;

use super::{InitContext, Module};

pub struct MorseCode {
    encoder: Mutex<MorseEncoder>,
}

impl MorseCode {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let dit = *ctx.args.get_one::<u64>("dit").unwrap();
        let frequency = *ctx.args.get_one::<f32>("frequency").unwrap();
        let text = ctx.args.get_one::<String>("text").unwrap();

        let mut encoder = MorseEncoder::new(ctx.sample_rate(), frequency, dit);
        encoder.add_data(text).unwrap();

        Arc::new(Self {
            encoder: Mutex::new(encoder),
        })
    }
}

impl Module for MorseCode {
    fn name(&self) -> &'static str {
        "morse-code"
    }

    fn output(&self, output: &mut [f32]) {
        let mut encoder = self.encoder.lock();
        for i in output.iter_mut() {
            *i = encoder.next().unwrap();
        }
    }
}
