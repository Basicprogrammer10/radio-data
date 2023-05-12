use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use afire::{
    trace::{self, Formatter, Level},
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
    data: Mutex<Vec<u8>>,
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
        trace::set_log_formatter(Logger);

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

        let mut new_data = BitVec::<u8, Lsb0>::new();
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

    pub fn get_raw(&self, len: usize) -> Option<Vec<u8>> {
        let mut data = self.data.lock();
        if data.len() < len {
            return None;
        }

        let out = data.drain(..len).collect();
        self.size.store(data.len(), Ordering::Release);
        Some(out)
    }
}

fn entropy(data: &[u8]) -> f32 {
    let mut counts = [0usize; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }

    let mut entropy = 0f32;
    for count in counts {
        if count == 0 {
            continue;
        }

        let p = count as f32 / data.len() as f32;
        entropy -= p * p.log2();
    }

    entropy // (data.len() as f32).log2()
}

struct Logger;

impl Formatter for Logger {
    fn format(&self, level: Level, color: bool, msg: String) {
        println!(
            "[L] {}{msg}{}",
            if color { level.get_color() } else { "" },
            if color { "\x1b[0m" } else { "" }
        );
    }
}

mod routes {
    use afire::{Content, Method, Response, Server};
    use serde::Serialize;

    use super::{entropy, TrueRandom};

    #[derive(Serialize)]
    struct Status {
        buffer_filled: usize,
        buffer_size: usize,
        percent_filled: f32,
        bit_ratio: f32,
        entropy: f32,
    }

    pub fn attach(server: &mut Server<TrueRandom>) {
        server.stateful_route(Method::GET, "/status", |app, _req| {
            let mut bit_ones = 0;
            let buffer = app.buffer.data.lock();
            let bits = buffer.len() * 8;
            for i in buffer.iter() {
                bit_ones += i.count_ones();
            }
            drop(buffer);

            let status = Status {
                buffer_filled: app.buffer.size(),
                buffer_size: app.args.buffer_size,
                percent_filled: app.buffer.size() as f32 / app.args.buffer_size as f32,
                bit_ratio: bit_ones as f32 / bits as f32,
                entropy: entropy(&app.buffer.data.lock()),
            };

            Response::new()
                .text(serde_json::to_string(&status).unwrap())
                .content(Content::JSON)
        });

        server.stateful_route(Method::GET, "/raw/{len}", |app, req| {
            let len = req.param("len").unwrap().parse::<usize>().unwrap();
            if len > app.buffer.size() {
                return Response::new()
                    .status(400)
                    .text("Buffer not filled enough.");
            }

            let data = app.buffer.get_raw(len).unwrap();
            let entropy = entropy(&data);
            Response::new()
                .bytes(&data)
                .header("X-Entropy", entropy.to_string())
        });

        server.stateful_route(Method::GET, "/data/number/{min}/{max}", |app, req| {
            let min = req.param("min").unwrap().parse::<f64>().unwrap();
            let max = req.param("max").unwrap().parse::<f64>().unwrap();

            let data = app.buffer.get_raw(8).unwrap();
            let entropy = entropy(&data);
            let number = u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]) as f64;
            let number = number / u64::MAX as f64 * (max - min) + min;

            Response::new()
                .text(number)
                .content(Content::JSON)
                .header("X-Entropy", entropy.to_string())
        });

        server.stateful_route(Method::GET, "/data/integer/{min}/{max}", |app, req| {
            let min = req.param("min").unwrap().parse::<u64>().unwrap() as f32;
            let max = req.param("max").unwrap().parse::<u64>().unwrap() as f32;

            let data = app.buffer.get_raw(8).unwrap();
            let entropy = entropy(&data);
            let number = u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]) as f32;
            let number = (number / u64::MAX as f32) * (max - min) + min;

            Response::new()
                .text(number)
                .content(Content::JSON)
                .header("X-Entropy", entropy.to_string())
        });
    }
}
