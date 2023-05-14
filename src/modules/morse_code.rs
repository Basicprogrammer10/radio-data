//! Morse code module.
//! Currently a work in progress.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::coding::morse::MorseEncoder;

use super::{InitContext, Module};

pub struct MorseCode {
    ctx: InitContext,
    encoder: Mutex<MorseEncoder>,
}

impl MorseCode {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        // Load command line arguments
        let dit = *ctx.args.get_one::<u64>("dit").unwrap();
        let frequency = *ctx.args.get_one::<f32>("frequency").unwrap();
        let text = ctx.args.get_one::<String>("text").unwrap();

        // Create the morse encoder and add the data
        let mut encoder = MorseEncoder::new(ctx.sample_rate(), frequency, dit);
        encoder.add_data(text).unwrap();

        Arc::new(Self {
            ctx,
            encoder: Mutex::new(encoder),
        })
    }
}

impl Module for MorseCode {
    fn name(&self) -> &'static str {
        "morse-code"
    }

    fn output(&self, output: &mut [f32]) {
        // Just pass the data from the encoder to the output of each channel
        let mut encoder = self.encoder.lock();
        let mut last = 0.0;
        for (i, e) in output.iter_mut().enumerate() {
            if i % self.ctx.output.channels() as usize == 0 {
                last = encoder.next().unwrap_or(last);
            }

            *e = last;
        }
    }
}
