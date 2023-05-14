//! Receive DTMF tones and decode them into binary data.

use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    coding::dtmf::{self, DtmfDecoder},
    consts::DTMF_CHUNK,
};

use super::{InitContext, Module};

pub struct DtmfReceive {
    ctx: InitContext,
    decode: Mutex<Option<DtmfDecoder>>,
    work: Mutex<Vec<f32>>,
    history: Mutex<Vec<u8>>,
}

impl DtmfReceive {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let out = Arc::new(Self {
            decode: Mutex::new(None),
            work: Mutex::new(Vec::new()),
            history: Mutex::new(Vec::new()),
            ctx,
        });

        // Create a new DTMF decoder and set its callback to self.callback
        let this = out.clone();
        *out.decode.lock() = Some(DtmfDecoder::new(out.ctx.sample_rate(), move |x| {
            this.callback(x as char)
        }));

        out
    }

    /// THis function is called when a byte is decoded.
    fn callback(&self, chr: char) {
        println!("[*] Got code: {chr}");

        // Add the byte to the history
        let mut history = self.history.lock();
        history.push(chr as u8);

        // If the history is long enough try to find the start and end codes
        // If these are found, decode the data and print it
        if history.len() > 2 && &history[history.len() - 2..] == b"#D" {
            println!("[*] Transmission Complete");
            let start = match history.windows(2).rposition(|x| x == b"A#") {
                Some(i) => i,
                None => {
                    println!("[-] Start code not found");
                    return;
                }
            };

            let raw = dtmf::dtmf_to_bin(&history[start + 2..&history.len() - 2]);
            println!(" \\ {}", raw.iter().map(|x| *x as char).collect::<String>());
            history.clear();
        }
    }
}

impl Module for DtmfReceive {
    fn name(&self) -> &'static str {
        "DtmfReceive"
    }

    fn input(&self, input: &[f32]) {
        // Add the input to the work buffer
        let mut work = self.work.lock();
        work.extend(
            input
                .iter()
                .enumerate()
                .filter(|x| x.0 % self.ctx.input.channels() as usize == 0)
                .map(|x| *x.1),
        );

        // If the data is at least DTMF_CHUNK long, process it
        for _ in 0..work.len() / DTMF_CHUNK {
            let chunk = work.drain(..DTMF_CHUNK).collect::<Vec<_>>();
            self.decode.lock().as_mut().unwrap().process(&chunk);
        }
    }
}
