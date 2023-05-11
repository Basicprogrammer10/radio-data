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
use parking_lot::Mutex;

use super::{InitContext, Module};

pub struct TrueRandom {
    args: Args,
    buffer: Buffer,
}

struct Buffer {
    data: Mutex<Vec<f32>>,
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
        // Todo:: this
        let mut buffer = self.data.lock();
        buffer.extend_from_slice(data);
        self.size.store(buffer.len(), Ordering::Release);
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
    }

    pub fn attach(server: &mut Server<TrueRandom>) {
        server.stateful_route(Method::GET, "/status", |app, _req| {
            let status = Status {
                buffer_filled: app.buffer.size(),
                buffer_size: app.args.buffer_size,
            };

            Response::new()
                .text(serde_json::to_string(&status).unwrap())
                .content(Content::JSON)
        });
    }
}
