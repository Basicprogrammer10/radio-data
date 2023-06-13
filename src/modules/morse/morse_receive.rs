use std::{
    io::{self, Write},
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

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
    last_state: AtomicBool,
}

impl MorseReceive {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        // Load command line arguments
        let dit = *ctx.args.get_one::<u64>("dit").unwrap();
        let frequency = *ctx.args.get_one::<f32>("frequency").unwrap();

        // Create the morse decoder
        let decoder = MorseDecoder::new(ctx.sample_rate(), frequency, dit, |c| {
            let mut bytes = [0; 4];
            c.encode_utf8(&mut bytes);

            let mut stdout = io::stdout();
            stdout.write_all(&bytes).unwrap();
            stdout.flush().unwrap();
        });

        Arc::new(Self {
            ctx,
            decoder: Mutex::new(decoder),
            buffer: Mutex::new(Vec::new()),
            last_state: AtomicBool::new(true),
        })
    }
}

impl Module for MorseReceive {
    fn name(&self) -> &'static str {
        "morse-receive"
    }

    fn init(&self) {
        println!();
    }

    fn input(&self, input: &[f32]) {
        let is_idle = self.decoder.lock().is_idle();
        if !self.last_state.swap(is_idle, Ordering::Relaxed) && is_idle {
            process::exit(0);
        }

        let channels = self.ctx.input.channels() as usize;
        let mut buffer = self.buffer.lock();
        buffer.extend(
            input
                .iter()
                .enumerate()
                .filter(|(i, _)| i % channels == 0)
                .map(|(_, e)| e),
        );

        let mut decoder = self.decoder.lock();
        for _ in 0..buffer.len() / MORSE_CHUNK {
            decoder.process(&buffer[..MORSE_CHUNK]);
            buffer.drain(..MORSE_CHUNK);
        }
    }
}
