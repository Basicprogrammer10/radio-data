use std::{
    io::{stdout, Write},
    sync::Arc,
};

use crossterm::{execute, queue, style, terminal};
use parking_lot::Mutex;
use rustfft::{num_complex::Complex, FftPlanner};

use super::{InitContext, Module};

const FFT_SAMPLE_SIZE: usize = 1024 * 2;
const COLOR_SCHEME: [Color; 2] = [Color::hex(0x05071c), Color::hex(0xf9e9da)];

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
        for i in data {
            vals.push(*i);

            if vals.len() as f32 > points_per_char {
                let avg = vals.iter().sum::<f32>() / vals.len() as f32;
                let color = color(avg);

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

            self.print_row(
                &buf[..buf.len() / 2]
                    .iter()
                    .map(|x| x.norm())
                    .collect::<Vec<_>>(),
            );
        }
    }
}

fn color(val: f32) -> Color {
    COLOR_SCHEME[0].lerp(&COLOR_SCHEME[1], val)
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
