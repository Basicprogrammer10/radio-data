use std::{
    f32::consts::{E, PI},
    io::{stdout, Write},
    ops::Range,
    sync::Arc,
};

use crossterm::{execute, queue, style, terminal};
use parking_lot::Mutex;
use rustfft::{num_complex::Complex, FftPlanner};

use super::{InitContext, Module};

// todo: allow setting these with command line arguments
const FFT_SAMPLE_SIZE: usize = 1024 * 2;
const DISPLAY_RANGE: Range<usize> = 15..14_000;
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
    planner: Mutex<FftPlanner<f32>>,
    samples: Mutex<Vec<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        execute!(
            stdout(),
            terminal::EnterAlternateScreen,
            terminal::Clear(terminal::ClearType::All)
        )
        .unwrap();

        Arc::new(Self {
            ctx,
            planner: Mutex::new(FftPlanner::<f32>::new()),
            samples: Mutex::new(Vec::with_capacity(FFT_SAMPLE_SIZE)),
        })
    }

    fn print_row(&self, data: &[f32]) {
        let mut stdout = stdout();
        let console_width = terminal::size().unwrap().0;
        let bar_width = console_width as usize / data.len();
        let points_per_char = data.len() as f32 / console_width as f32;

        let mut vals = Vec::new();
        let mut error = 0.;
        for i in data {
            vals.push(*i);

            if vals.len() as f32 + error >= points_per_char {
                error = vals.len() as f32 + error - points_per_char;
                let avg = vals.iter().sum::<f32>() / vals.len() as f32;
                let color = color(1. - E.powf(-avg));

                let bar = "█".repeat(if bar_width > 0 { bar_width } else { 1 });
                queue!(
                    stdout,
                    style::SetForegroundColor(style::Color::Rgb {
                        r: color.r,
                        g: color.g,
                        b: color.b
                    }),
                    style::Print(bar),
                )
                .unwrap();

                vals.clear();
            }
        }

        queue!(stdout, style::ResetColor, style::Print("\n")).unwrap();
        stdout.flush().unwrap();
    }
}

impl Module for SpectrumAnalyzer {
    fn name(&self) -> &'static str {
        "spectrum_analyzer"
    }

    fn init(&self) {
        let resolution = 1. / FFT_SAMPLE_SIZE as f32 * self.ctx.sample_rate().input as f32;
        println!("[*] Resolution: {} Hz", resolution);
    }

    fn input(&self, input: &[f32]) {
        let mut samples = self.samples.lock();
        samples.extend_from_slice(input);

        while samples.len() >= FFT_SAMPLE_SIZE {
            let mut buf = Vec::with_capacity(FFT_SAMPLE_SIZE);
            for i in samples.drain(..FFT_SAMPLE_SIZE) {
                buf.push(Complex::new(i, 0.));
            }

            let fft = self.planner.lock().plan_fft_forward(FFT_SAMPLE_SIZE);
            fft.process(&mut buf);

            let sample_rate = self.ctx.sample_rate().input as usize;
            let start = DISPLAY_RANGE.start * FFT_SAMPLE_SIZE / sample_rate;
            let end = DISPLAY_RANGE.end * FFT_SAMPLE_SIZE / sample_rate;

            self.print_row(&hann_window(
                &buf[start.max(0)..=end.min(buf.len() / 2)]
                    .iter()
                    .map(|x| x.norm())
                    .collect::<Vec<_>>(),
            ));
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
