use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use parking_lot::Mutex;

use crate::{
    audio::tone::Tone,
    coding::dtmf::{self, DtmfEncoder},
    modules::{InitContext, Module},
};

pub struct DtmfSend {
    ctx: InitContext,
    state: Mutex<State>,
    encode: Mutex<DtmfEncoder>,
    i: AtomicUsize,
}

/// The state of the DTMF sender
enum State {
    /// Sending data from the [`DtmfEncoder`]
    Transmitting,
    /// Sending a start tone.
    /// This is needed because the VOX setting on my radio takes a second to activate.
    /// So this tone allows the radio to activate before sending the data.
    Head(Tone),
}

impl DtmfSend {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let sr = ctx.sample_rate();

        // Convert the data to DTMF
        let to_send = ctx
            .args
            .subcommand()
            .unwrap()
            .1
            .get_one::<String>("data")
            .unwrap();
        let mut to_send = dtmf::bin_to_dtmf(to_send.as_bytes());

        // Add the start and end codes
        // I don't remember what these codes mean in binary, but they should probably be changed
        to_send.insert(0, b'A');
        to_send.insert(1, b'#');
        to_send.push(b'#');
        to_send.push(b'D');

        // Prints the DTMF encoded data
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
                // After one second of sending the HEAD tone, start sending the data
                // Note: This probably shouldn't be run if the state is already transmitting
                if self.i.fetch_add(1, Ordering::Relaxed) > self.ctx.input.sample_rate().0 as usize
                {
                    *self.state.lock() = State::Transmitting;
                }

                // Get the next sample from either the HEAD tone or the DTMF encoder
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
