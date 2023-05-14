use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    coding::morse::MorseDecoder,
    modules::{InitContext, Module},
};

const MORSE_CHUNK: usize = 512;

pub struct MorseReceive {
    ctx: InitContext,
    decoder: Mutex<MorseDecoder>,
    buffer: Mutex<Vec<f32>>,
}

impl MorseReceive {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        // Load command line arguments
        let dit = *ctx.args.get_one::<u64>("dit").unwrap();
        let frequency = *ctx.args.get_one::<f32>("frequency").unwrap();

        // Create the morse decoder
        let decoder = MorseDecoder::new(ctx.sample_rate(), frequency, dit, |c| println!("{}", c));

        Arc::new(Self {
            ctx,
            decoder: Mutex::new(decoder),
            buffer: Mutex::new(Vec::new()),
        })
    }
}

impl Module for MorseReceive {
    fn name(&self) -> &'static str {
        "morse-receive"
    }

    fn input(&self, input: &[f32]) {
        let channels = self.ctx.input.channels() as usize;
        let mut buffer = self.buffer.lock();
        buffer.extend(
            input
                .iter()
                .enumerate()
                .filter(|(i, _)| i % channels == 0)
                .map(|(_, e)| e),
        );

        for _ in 0..buffer.len() / MORSE_CHUNK {
            let mut decoder = self.decoder.lock();
            decoder.process(&buffer[..MORSE_CHUNK]);
            buffer.drain(..MORSE_CHUNK);
        }
    }
}
