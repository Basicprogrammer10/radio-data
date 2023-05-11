//! Spectrum analyzer module that uses the terminal.
//!
//! For future reference:
//! https://docs.rs/spectrum-analyzer/latest/src/spectrum_analyzer/windows.rs.html

use std::{
    f32::consts::{E, PI},
    io::{stdout, Write},
    ops::Range,
    process,
    sync::Arc,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute, queue, style, terminal,
};
use parking_lot::Mutex;
use rustfft::{num_complex::Complex, FftPlanner};

use super::{InitContext, Module};

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
    display_range: Range<usize>,

    // todo: store the FFT here instead of the planner
    planner: Mutex<FftPlanner<f32>>,
    samples: Mutex<Vec<f32>>,
    last_samples: Mutex<Option<Vec<f32>>>,
}

impl SpectrumAnalyzer {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let fft_size = *ctx.args.get_one("fft-size").unwrap();
        let display_range = ctx
            .args
            .get_one::<Range<usize>>("display-range")
            .unwrap()
            .to_owned();

        Arc::new(Self {
            ctx,
            fft_size,
            display_range,
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

        let mut stdout = stdout();
        let console_size = terminal::size().unwrap();
        let bar_width = console_size.0 as usize / data.len();
        let points_per_char = data.len() as f32 / console_size.0 as f32;

        queue!(
            stdout,
            cursor::MoveTo(0, 1),
            terminal::Clear(terminal::ClearType::CurrentLine),
            style::Print(self.top_line(console_size)),
            cursor::MoveTo(0, console_size.1),
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
                let width = if bar_width > 0 { bar_width } else { 1 };
                let bar = "▀".repeat(width);
                full_size += width;

                queue!(
                    stdout,
                    style::SetForegroundColor(get_color(&vals, |x| x.0).into()),
                    style::SetBackgroundColor(get_color(&vals, |x| x.1).into()),
                    style::Print(bar),
                )
                .unwrap();
                vals.clear();

                if full_size % 5 == 0 {
                    freq_labels.push((full_size, self.index_to_freq(i)));
                }
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

        let mut bottom_line = " ".repeat(console_size.0 as usize);
        for i in freq_labels {
            // bottom_line
        }

        queue!(
            stdout,
            style::ResetColor,
            terminal::ScrollUp(1),
            cursor::MoveToColumn(0),
            style::Print(bottom_line),
        )
        .unwrap();
        stdout.flush().unwrap();
        *last_samples = None;
    }

    fn handle_key_events(&self) {
        let event = event::poll(Duration::ZERO).unwrap();
        if !event {
            return;
        }

        match event::read().unwrap() {
            event::Event::Key(e) => match e.code {
                KeyCode::Esc => {
                    execute!(
                        stdout(),
                        terminal::LeaveAlternateScreen,
                        terminal::EnableLineWrap,
                        cursor::Show
                    )
                    .unwrap();
                    terminal::disable_raw_mode().unwrap();
                    process::exit(0);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn top_line(&self, size: (u16, u16)) -> String {
        let start = format!("[RADIO-DATA SPECTRUM ANALYZER]");
        let end = format!(
            "{{FFT size: {}, Display range {:?}}} [ESC: Quit]",
            self.fft_size, self.display_range
        );

        let diff = (size.0 as usize).saturating_sub(start.len() + end.len());
        format!("{}{}{}", start, " ".repeat(diff), end)
    }

    fn index_to_freq(&self, idx: usize) -> f32 {
        idx as f32 * self.ctx.sample_rate().input as f32 / self.fft_size as f32
    }
}

impl Module for SpectrumAnalyzer {
    fn name(&self) -> &'static str {
        "spectrum_analyzer"
    }

    fn init(&self) {
        let resolution = 1. / self.fft_size as f32 * self.ctx.sample_rate().input as f32;
        println!("[I] FFT size: {}", self.fft_size);
        println!("[I] Display range: {:?}", self.display_range);
        println!("[I] Resolution: {resolution} Hz");

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
        let mut samples = self.samples.lock();
        samples.extend_from_slice(input);

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
