//! Spectrum analyzer module that uses the terminal.
//!
//! For future reference:
//! - https://phip1611.de/blog/frequency-spectrum-analysis-with-fft-in-rust/
//! - https://www.sjsu.edu/people/burford.furman/docs/me120/FFT_tutorial_NI.pdf
//! - https://www.youtube.com/watch?v=dCeHOf4cJE0
//! - https://docs.rs/spectrum-analyzer/latest/src/spectrum_analyzer/windows.rs.html

use std::{
    collections::VecDeque,
    f32::consts::{E, PI},
    io::{stdout, Write},
    ops::Range,
    panic, process,
    sync::Arc,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute, queue, style, terminal,
};
use parking_lot::Mutex;
use rubato::{InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction};
use rustfft::{num_complex::Complex, FftPlanner};

use crate::misc::buf_writer::BufWriter;

use super::{InitContext, Module};

const FREQUENCY_UNITS: &[&str] = &["Hz", "kHz", "MHz", "GHz", "THz"];
const COLOR_SCHEME: &[Color] = &[
    Color::hex(0x000000),
    Color::hex(0x742975),
    Color::hex(0xDD562E),
    Color::hex(0xFD9719),
    Color::hex(0xFFD76B),
    Color::hex(0xFFFFFF),
];

pub struct SpectrumAnalyzer {
    ctx: InitContext,
    fft_size: usize,
    resolution: f32,
    display_range: Range<usize>,

    passthrough: Option<Mutex<PassThrough>>,
    planner: Mutex<FftPlanner<f32>>,
    samples: Mutex<Vec<f32>>,
    last_samples: Mutex<Option<Vec<f32>>>,
}

struct PassThrough {
    ctx: InitContext,
    resample_size: usize,
    resampler: SincFixedIn<f32>,
    buffer: Vec<VecDeque<f32>>,
    out_buffer: Vec<VecDeque<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let fft_size = *ctx.args.get_one("fft-size").unwrap();
        let display_range = ctx
            .args
            .get_one::<Range<usize>>("display-range")
            .unwrap()
            .to_owned();
        let passthrough = ctx
            .args
            .get_flag("passthrough")
            .then(|| Mutex::new(PassThrough::new(ctx.clone(), fft_size)));

        Arc::new(Self {
            resolution: 1. / fft_size as f32 * ctx.sample_rate().input as f32,
            ctx,
            fft_size,
            display_range,

            passthrough,
            planner: Mutex::new(FftPlanner::<f32>::new()),
            samples: Mutex::new(Vec::with_capacity(fft_size)),
            last_samples: Mutex::new(None),
        })
    }

    fn print_row(&self, data: Vec<f32>) {
        self.handle_key_events();

        let mut last_samples = self.last_samples.lock();
        if last_samples.is_none() {
            *last_samples = Some(data);
            return;
        }

        let mut stdout = BufWriter::new(stdout());
        let console_size = terminal::size().unwrap();
        let bar_width = console_size.0 as usize / data.len();
        let points_per_char = data.len() as f32 / console_size.0 as f32;

        queue!(
            stdout,
            terminal::ScrollUp(1),
            cursor::MoveTo(0, 0),
            style::Print(self.top_line(console_size, points_per_char)),
            cursor::MoveTo(0, console_size.1.saturating_sub(2)),
        )
        .unwrap();

        let mut vals = Vec::new();
        let mut freq_labels = Vec::new();
        let mut error = 0.;
        let mut full_size = 0;

        let prev_data = last_samples.as_ref().unwrap().iter().copied();
        for (i, e) in data.into_iter().zip(prev_data).enumerate() {
            vals.push(e);

            if vals.len() as f32 + error >= points_per_char {
                error = vals.len() as f32 + error - points_per_char;
                freq_labels.push((full_size, self.index_to_freq(i)));

                let width = if bar_width > 0 { bar_width } else { 1 };
                let bar = "▀".repeat(width);
                full_size += width;

                queue!(
                    stdout,
                    style::SetForegroundColor(get_color(&vals, |x| x.1).into()),
                    style::SetBackgroundColor(get_color(&vals, |x| x.0).into()),
                    style::Print(bar),
                )
                .unwrap();
                vals.clear();
            }
        }

        if console_size.0 as usize > full_size {
            queue!(
                stdout,
                style::SetForegroundColor(COLOR_SCHEME[0].into()),
                style::SetBackgroundColor(COLOR_SCHEME[0].into()),
                style::Print("▀".repeat(console_size.0 as usize - full_size)),
            )
            .unwrap();
        }

        queue!(stdout, style::ResetColor, cursor::MoveDown(1)).unwrap();
        let mut i = 0;
        while i < freq_labels.len() {
            let val = &freq_labels[i];
            let freq = nice_freq(val.1);
            i += (freq.len() + 3) / bar_width.max(1);

            if val.0 + freq.len() >= console_size.0 as usize {
                break;
            }

            queue!(
                stdout,
                cursor::MoveToColumn(val.0 as u16),
                style::Print(format!("└{freq}")),
            )
            .unwrap();

            i += 1;
        }

        stdout.flush().unwrap();
        *last_samples = None;
    }

    fn handle_key_events(&self) {
        let event = event::poll(Duration::ZERO).unwrap();
        if !event {
            return;
        }

        match event::read().unwrap() {
            event::Event::Key(e) => {
                if e.code == KeyCode::Esc {
                    exit();
                    process::exit(0);
                }
            }
            event::Event::Resize(..) => {
                execute!(stdout(), terminal::Clear(terminal::ClearType::All)).unwrap()
            }
            _ => {}
        }
    }

    fn top_line(&self, size: (u16, u16), points_per_char: f32) -> String {
        let start = "[RADIO-DATA SPECTRUM ANALYZER]";
        let end = format!(
            "{{FFT size: {}, Domain: {}..{}, BinRes: {}, BinChars: {:.1}}} [ESC: Quit]",
            self.fft_size,
            nice_freq(self.display_range.start as f32),
            nice_freq(self.display_range.end as f32),
            nice_freq(self.resolution),
            points_per_char
        );

        let diff = (size.0 as usize).saturating_sub(start.len() + end.len());
        format!("{}{}{}", start, " ".repeat(diff), end)
    }

    fn index_to_freq(&self, idx: usize) -> f32 {
        idx as f32 * self.ctx.sample_rate().input as f32 / self.fft_size as f32
    }
}

impl PassThrough {
    fn new(ctx: InitContext, _fft_size: usize) -> Self {
        let resample_size = 1024;
        let channels = ctx.input.channels().min(ctx.output.channels()) as usize;
        let parameters = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = SincFixedIn::new(
            (ctx.sample_rate().output / ctx.sample_rate().input) as f64,
            2.,
            parameters,
            resample_size,
            channels,
        )
        .unwrap();

        Self {
            ctx,
            resampler,
            resample_size,
            buffer: vec![VecDeque::new(); channels],
            out_buffer: vec![VecDeque::new(); channels],
        }
    }

    fn add_samples(&mut self, samples: &[f32]) {
        let inp_channels = self.ctx.input.channels() as usize;
        let channels = self.buffer.len();

        for (i, &e) in samples.iter().enumerate() {
            let channel = i % inp_channels;
            if channel >= channels {
                continue;
            }

            self.buffer[channel].push_back(e);
        }

        while self.buffer.iter().map(|x| x.len()).max().unwrap_or(0) >= self.resample_size {
            let mut samples = vec![Vec::new(); channels];
            for _ in 0..self.resample_size {
                for (j, e) in samples.iter_mut().enumerate().take(channels) {
                    e.push(self.buffer[j].pop_front().unwrap_or(0.0));
                }
            }

            let out = self.resampler.process(&samples, None).unwrap();
            for (i, e) in out.into_iter().enumerate() {
                self.out_buffer[i].extend(e);
            }
        }
    }

    fn write_output(&mut self, output: &mut [f32]) {
        let out_channels = self.ctx.output.channels() as usize;

        for (i, e) in output.iter_mut().enumerate() {
            let channel = i % self.ctx.output.channels() as usize;
            if channel >= out_channels {
                *e = 0.0;
                continue;
            }

            *e = self.out_buffer[channel].pop_front().unwrap_or(0.0);
        }
    }
}

impl Module for SpectrumAnalyzer {
    fn name(&self) -> &'static str {
        "spectrum_analyzer"
    }

    fn init(&self) {
        println!("[I] FFT size: {}", self.fft_size);
        println!("[I] Display range: {:?}", self.display_range);
        println!("[I] Resolution: {}", nice_freq(self.resolution));

        panic::set_hook(Box::new(|info| {
            exit();
            eprintln!("{info}");
            process::exit(0)
        }));

        terminal::enable_raw_mode().unwrap();

        let height = terminal::size().unwrap().1;
        execute!(
            stdout(),
            terminal::EnterAlternateScreen,
            terminal::DisableLineWrap,
            cursor::Hide,
            cursor::MoveToRow(height)
        )
        .unwrap();
    }

    fn input(&self, input: &[f32]) {
        if let Some(i) = &self.passthrough {
            i.lock().add_samples(input);
        }

        let mut samples = self.samples.lock();
        samples.reserve(input.len() / self.ctx.input.channels() as usize + 1);

        let mut working = 0.0;
        for (i, e) in input.iter().enumerate() {
            working += e;

            if i != 0 && i % self.ctx.input.channels() as usize == 0 {
                samples.push(working / self.ctx.input.channels() as f32);
                working = 0.0;
            }
        }
        samples.push(working / self.ctx.input.channels() as f32);

        while samples.len() >= self.fft_size {
            let mut buf = Vec::with_capacity(self.fft_size);
            for i in samples.drain(..self.fft_size) {
                buf.push(Complex::new(i, 0.));
            }

            let fft = self.planner.lock().plan_fft_forward(self.fft_size);
            fft.process(&mut buf);

            let sample_rate = self.ctx.sample_rate().input as usize;
            let start = self.display_range.start * self.fft_size / sample_rate;
            let end = self.display_range.end * self.fft_size / sample_rate;

            self.print_row(hann_window(
                &buf[start.max(0)..=end.min(buf.len() / 2)]
                    .iter()
                    .map(|x| x.norm())
                    .collect::<Vec<_>>(),
            ));
        }
    }

    fn output(&self, output: &mut [f32]) {
        if let Some(i) = &self.passthrough {
            i.lock().write_output(output);
        }
    }
}

fn hann_window(samples: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples.len());
    for (i, e) in samples.iter().enumerate() {
        let a = 2.0 * PI * i as f32;
        let n = (a / samples.len() as f32).cos();
        let m = 0.5 * (1.0 - n);
        out.push(m * e)
    }
    out
}

fn color(val: f32) -> Color {
    let val = val.max(0.).min(1.);
    let sections = COLOR_SCHEME.len() - 2;
    let section = (sections as f32 * val).floor() as usize;

    COLOR_SCHEME[section].lerp(
        &COLOR_SCHEME[section + 1],
        val * sections as f32 - section as f32,
    )
}

fn get_color(vals: &[(f32, f32)], map: impl Fn(&(f32, f32)) -> f32) -> Color {
    let avg = vals.iter().map(map).sum::<f32>() / vals.len() as f32;
    let norm = 1. - E.powf(-avg);

    color(norm)
}

fn nice_freq(mut hz: f32) -> String {
    for i in FREQUENCY_UNITS {
        if hz < 1000. {
            return format!("{:.1}{}", hz, i);
        }

        hz /= 1000.;
    }

    format!("{:.1}{}", hz, FREQUENCY_UNITS.last().unwrap())
}

fn exit() {
    execute!(
        stdout(),
        terminal::LeaveAlternateScreen,
        terminal::EnableLineWrap,
        cursor::Show
    )
    .unwrap();
    terminal::disable_raw_mode().unwrap();
}

#[derive(Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    const fn hex(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }

    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        )
    }
}

impl From<Color> for style::Color {
    fn from(color: Color) -> Self {
        style::Color::Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}
