use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use afire::{
    trace::{self, Level},
    Server,
};
use bitvec::{order::Lsb0, vec::BitVec, view::BitView};
use parking_lot::Mutex;

use super::{InitContext, Module};

pub struct TrueRandom {
    args: Args,
    buffer: Buffer,
}

struct Buffer {
    target: usize,
    data: Mutex<Vec<usize>>,
    size: AtomicUsize,
}

struct Args {
    host: String,
    port: u16,
    threads: usize,
    buffer_size: usize,
}

impl TrueRandom {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        trace::set_log_level(Level::Trace);

        let args = Args {
            host: ctx.args.get_one::<String>("host").unwrap().to_owned(),
            port: *ctx.args.get_one("port").unwrap(),
            threads: *ctx.args.get_one("threads").unwrap(),
            buffer_size: *ctx.args.get_one("buffer-size").unwrap(),
        };

        let this = Self {
            buffer: Buffer::new(args.buffer_size),
            args,
        };

        let mut server = Server::<Self>::new(&this.args.host, this.args.port).state(this);
        routes::attach(&mut server);

        let app = server.app();
        let threads = app.args.threads;
        thread::spawn(move || server.start_threaded(threads).unwrap());

        app
    }
}

impl Module for TrueRandom {
    fn name(&self) -> &'static str {
        "true-random"
    }

    fn input(&self, input: &[f32]) {
        if self.buffer.size() >= self.args.buffer_size {
            return;
        }

        self.buffer.fill_buffer(input);
    }
}

impl Buffer {
    pub fn new(size: usize) -> Self {
        Self {
            target: size,
            data: Mutex::new(Vec::with_capacity(size)),
            size: AtomicUsize::new(0),
        }
    }

    /// Get the number of samples in the buffer.
    pub fn size(&self) -> usize {
        self.size.load(Ordering::Acquire)
    }

    /// Fill the buffer with the given data.
    /// Will not necessarily use all of the data or fill the buffer completely.
    pub fn fill_buffer(&self, data: &[f32]) {
        let needed = self.target.saturating_sub(self.size());
        if needed == 0 {
            return;
        }

        let mut new_data = BitVec::<usize, Lsb0>::new();
        for &sample in data {
            let bits = sample.to_bits();
            new_data.extend(
                bits.view_bits::<Lsb0>()
                    .chunks(2)
                    .filter(|x| x[0] != x[1])
                    .map(|x| x[0]),
            );

            if new_data.len() >= needed {
                break;
            }
        }

        let new_data = new_data.into_vec();
        let mut data = self.data.lock();
        data.extend(new_data[..needed.min(new_data.len())].into_iter());
        self.size.store(data.len(), Ordering::Release);
    }
}

mod routes {
    use afire::{Content, Method, Response, Server};
    use serde::Serialize;

    use super::TrueRandom;

    #[derive(Serialize)]
    struct Status {
        buffer_filled: usize,
        buffer_size: usize,
        percent_filled: f32,
    }

    pub fn attach(server: &mut Server<TrueRandom>) {
        server.stateful_route(Method::GET, "/status", |app, _req| {
            let status = Status {
                buffer_filled: app.buffer.size(),
                buffer_size: app.args.buffer_size,
                percent_filled: app.buffer.size() as f32 / app.args.buffer_size as f32,
            };

            Response::new()
                .text(serde_json::to_string(&status).unwrap())
                .content(Content::JSON)
        });
    }
}
