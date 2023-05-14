//! True random number generator (TRNG) module that uses atmospheric noise.
//! It hosts a web server (with afire) to allow other applications to get random numbers.

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
    ctx: InitContext,
    args: Args,
    buffer: Buffer,
}

/// Buffer of random data
struct Buffer {
    target: usize,
    data: Mutex<Vec<u8>>,
    size: AtomicUsize,
}

// Arguments for this module
struct Args {
    host: String,
    port: u16,
    threads: usize,
    buffer_size: usize,
}

impl TrueRandom {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        // Setup afire's tracing
        trace::set_log_level(Level::Trace);
        trace::set_log_formatter(Logger);

        // Load command line arguments
        let args = Args {
            host: ctx.args.get_one::<String>("host").unwrap().to_owned(),
            port: *ctx.args.get_one("port").unwrap(),
            threads: *ctx.args.get_one("threads").unwrap(),
            buffer_size: *ctx.args.get_one("buffer-size").unwrap(),
        };

        let this = Self {
            ctx,
            buffer: Buffer::new(args.buffer_size),
            args,
        };

        // Create a new web server
        let mut server = Server::<Self>::new(&this.args.host, this.args.port).state(this);
        routes::attach(&mut server);

        // Start the server in a new thread
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
        // If the buffer is full, don't add any more data
        if self.buffer.size() >= self.args.buffer_size {
            return;
        }

        // Add the data to the buffer.
        // If you have more than one channel, the data will be averaged.
        let mut buffer = Vec::with_capacity(input.len() / self.ctx.input.channels() as usize + 1);
        let mut working = 0.0;
        for (i, e) in input.iter().enumerate() {
            working += e;

            if i != 0 && i % self.ctx.input.channels() as usize == 0 {
                buffer.push(working / self.ctx.input.channels() as f32);
                working = 0.0;
            }
        }
        buffer.push(working / self.ctx.input.channels() as f32);

        // Convert data from a float in the range [-1, 1] to an i32 in the range [-2^31, 2^31)
        self.buffer.fill_buffer(
            &buffer
                .iter()
                .map(|x| (x * i32::MAX as f32) as i32)
                .collect::<Vec<_>>(),
        );
    }
}

impl Buffer {
    /// Create a new buffer with the given size.
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
    pub fn fill_buffer(&self, data: &[i32]) {
        // If no more data is needed, don't add any more
        let needed = self.target.saturating_sub(self.size());
        if needed == 0 {
            return;
        }

        // This is for correcting for any bias in the data.
        // It does this by going through the data in chunks of two bits, if both bits are the same, it discards them.
        // Then it takes the first bit of each chunk and adds it to the buffer.
        // This discards a lot but its important for getting rid of bias.
        let mut new_data = BitVec::<u8, Lsb0>::new();
        for &sample in data {
            new_data.extend(
                sample
                    .to_ne_bytes()
                    .view_bits::<Lsb0>()
                    .chunks(2)
                    .filter(|x| x[0] != x[1])
                    .map(|x| x[0]),
            );

            // If we have enough data, stop
            if new_data.len() >= needed {
                break;
            }
        }

        // Convert the data to a vector of bytes and add it to the buffer
        let new_data = new_data.into_vec();
        let mut data = self.data.lock();
        data.extend(new_data[..needed.min(new_data.len())].iter());
        self.size.store(data.len(), Ordering::Release);
    }

    // Get the specified number of bytes from the buffer.
    // If the buffer doesn't have enough data, return None.
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

/// Calculate the entropy of the given data.
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

/// Custom logger for afire
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

/// Define the routes for the server
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
        // Status endpoint, which returns the status of the buffer including entropy and bit ratio
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

        // Get a specified number of bytes from the buffer
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

        // Gets a random float between {min} and {max}
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

        // Gets a random integer between {min} and {max}
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
