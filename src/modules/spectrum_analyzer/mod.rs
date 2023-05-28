//! Spectrum analyzer module that uses the terminal.
//!
//! For future reference:
//! - <https://phip1611.de/blog/frequency-spectrum-analysis-with-fft-in-rust/>
//! - <https://www.sjsu.edu/people/burford.furman/docs/me120/FFT_tutorial_NI.pdf>
//! - <https://www.youtube.com/watch?v=dCeHOf4cJE0>
//! - <https://docs.rs/spectrum-analyzer/latest/src/spectrum_analyzer/windows.rs.html>

use std::{f32::consts::E, ops::Range, sync::Arc, thread};

use clap::ValueEnum;
use crossterm::style;
use num_complex::Complex;
use parking_lot::{Mutex, RwLock};
use rustfft::FftPlanner;

use super::{InitContext, Module};
use crate::audio::{algorithms::to_mono, passthrough::PassThrough, windows::BoxedWindow};

mod console;
mod window;

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
    gain: f32,
    display_range: Range<usize>,
    window: Arc<BoxedWindow>,

    passthrough: Option<Mutex<PassThrough>>,
    planner: Mutex<FftPlanner<f32>>,
    samples: Mutex<Vec<f32>>,
    last_samples: Mutex<Option<Vec<f32>>>,
    this: Mutex<Option<Arc<SpectrumAnalyzer>>>,

    renderer: RwLock<Option<Box<dyn Renderer + Send + Sync + 'static>>>,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum DisplayType {
    Console,
    Window,
}

trait Renderer {
    fn init(&self);
    fn render(&self, data: Vec<f32>);
    fn exit(&self);
}

impl SpectrumAnalyzer {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        // Load command line arguments
        let fft_size = *ctx.args.get_one("fft-size").unwrap();
        let display_range = ctx
            .args
            .get_one::<Range<usize>>("display-range")
            .unwrap()
            .to_owned();
        let passthrough = ctx
            .args
            .get_flag("passthrough")
            .then(|| Mutex::new(PassThrough::new(ctx.clone(), 1024)));
        let window = ctx
            .args
            .get_one::<Arc<BoxedWindow>>("window")
            .unwrap()
            .to_owned();
        let gain = *ctx.args.get_one("gain").unwrap();

        let renderer = *ctx
            .args
            .get_one::<DisplayType>("display-type")
            .unwrap_or(&DisplayType::Console);

        let this = Arc::new(Self {
            resolution: 1. / fft_size as f32 * ctx.sample_rate().input as f32,
            ctx,
            fft_size,
            display_range,
            window,
            gain,

            passthrough,
            planner: Mutex::new(FftPlanner::<f32>::new()),
            samples: Mutex::new(Vec::with_capacity(fft_size)),
            last_samples: Mutex::new(None),
            this: Mutex::new(None),

            renderer: RwLock::new(None),
        });

        let renderer = match renderer {
            DisplayType::Console => Box::new(console::ConsoleRenderer {
                analyzer: this.clone(),
            }),
            DisplayType::Window => todo!(),
        };

        this.renderer.write().replace(renderer);
        this.this.lock().replace(this.clone());
        this
    }

    /// Defines the top status line.
    /// This line contains some stats about the current state of the program:
    /// - FFT size &mdash; The number of samples that are used for each FFT.
    /// - Domain &mdash; The frequency range that is currently displayed.
    /// - Gain &mdash; The gain that is applied to the data when displaying.
    /// - Res &mdash; The frequency resolution of each character used to display the spectrum.
    /// - RMS &mdash; The Root Mean Square value of the current FFT data.
    fn top_line(&self, size: (u16, u16), points_per_char: f32, rms: f32) -> String {
        let start = "[RADIO-DATA SPECTRUM ANALYZER]";
        let end = format!(
            "{{FFT size: {}, Window: {}, Domain: {}..{}, Gain: {:.1}, Res: {}, RMS: {:.1}}} [ESC: Quit]",
            self.fft_size,
            self.window.name(),
            nice_freq(self.display_range.start as f32),
            nice_freq(self.display_range.end as f32),
            self.gain,
            nice_freq(self.resolution * points_per_char),
            rms
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
        // Prints some info about the current state of the program
        println!("[I] FFT size: {}", self.fft_size);
        println!("[I] Display range: {:?}", self.display_range);
        println!("[I] Resolution: {}", nice_freq(self.resolution));

        self.renderer.read().as_ref().unwrap().init();
    }

    fn input(&self, input: &[f32]) {
        // Add the buffer to the pass-through
        if let Some(i) = &self.passthrough {
            i.lock().add_samples(input);
        }

        // Multithread to make sure the audio passthrough is never blocked
        let input = input.to_vec();
        let this = self.this.lock().clone().unwrap();
        thread::spawn(move || {
            // Adds the samples to a buffer
            let mut samples = this.samples.lock();
            samples.reserve(input.len() / this.ctx.input.channels() as usize + 1);
            samples.extend(to_mono(&input, this.ctx.input.channels() as usize));

            // If the buffer is big enough, it will process it
            while samples.len() >= this.fft_size {
                // Applies the windowing function and converts the samples to complex numbers
                let samples = samples.drain(..this.fft_size);
                let mut buf = Vec::with_capacity(this.fft_size);
                for &i in this.window.window(samples.as_slice()).iter() {
                    buf.push(Complex::new(i, 0.));
                }

                // Run the FFT
                let fft = this.planner.lock().plan_fft_forward(this.fft_size);
                fft.process(&mut buf);

                // Slice the buffer to the display range
                let sample_rate = this.ctx.sample_rate().input as usize;
                let start = this.display_range.start * this.fft_size / sample_rate;
                let end = this.display_range.end * this.fft_size / sample_rate;

                // Normalize the complex numbers (r^2 + i^2)
                let norm = buf[start.max(0)..=end.min(buf.len() / 2)]
                    .iter()
                    .map(|x| x.norm())
                    .collect::<Vec<_>>();

                this.renderer.read().as_ref().unwrap().render(norm);
            }
        });
    }

    fn output(&self, output: &mut [f32]) {
        // Writes the output from the pass-through
        if let Some(i) = &self.passthrough {
            i.lock().write_output(output);
        }
    }
}

/// Converts a frequency in Hz to a nice string with a unit.
fn nice_freq(mut hz: f32) -> String {
    for i in FREQUENCY_UNITS {
        if hz < 1000. {
            return format!("{hz:.1}{i}");
        }

        hz /= 1000.;
    }

    format!("{hz:.1}{}", FREQUENCY_UNITS.last().unwrap())
}

/// Takes in a value between 0 and 1 and returns a color from the color scheme.
fn color(val: f32) -> Color {
    debug_assert!((0. ..=1.).contains(&val));
    let sections = COLOR_SCHEME.len() - 2;
    let section = (sections as f32 * val).floor() as usize;

    COLOR_SCHEME[section].lerp(
        &COLOR_SCHEME[section + 1],
        val * sections as f32 - section as f32,
    )
}

/// Takes in an array of values and returns a color based on the average of the values.
/// A map function is also passed in to allow for picking different channels.
/// This is used in the print_row function to get the color of the previous row and then the current row.
fn get_color(vals: &[(f32, f32)], map: impl Fn(&(f32, f32)) -> f32) -> Color {
    let avg = vals.iter().map(map).sum::<f32>() / vals.len() as f32;
    let norm = 1. - E.powf(-avg);

    color(norm)
}

/// RGB color
#[derive(Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    /// Creates a new color from RGB values
    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Creates a new color from a hex value (no alpha)
    const fn hex(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }

    /// Linearly interpolates between two colors.
    /// Used in the above color function
    fn lerp(&self, other: &Self, t: f32) -> Self {
        Self::new(
            (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        )
    }
}

/// Converts a Color to a crossterm::style::Color
impl From<Color> for style::Color {
    fn from(color: Color) -> Self {
        style::Color::Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}