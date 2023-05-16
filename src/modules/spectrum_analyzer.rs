//! Spectrum analyzer module that uses the terminal.
//!
//! For future reference:
//! - <https://phip1611.de/blog/frequency-spectrum-analysis-with-fft-in-rust/>
//! - <https://www.sjsu.edu/people/burford.furman/docs/me120/FFT_tutorial_NI.pdf>
//! - <https://www.youtube.com/watch?v=dCeHOf4cJE0>
//! - <https://docs.rs/spectrum-analyzer/latest/src/spectrum_analyzer/windows.rs.html>

// yikes this is a very long file :sweat_smile:

use std::{
    collections::VecDeque,
    f32::consts::E,
    io::{stdout, Write},
    ops::Range,
    panic, process,
    sync::Arc,
    thread,
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

use crate::{
    audio::{algorithms::to_mono, windows::BoxedWindow},
    misc::buf_writer::BufWriter,
};

use super::{InitContext, Module};

const HALF_CHAR: &str = "▀";
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
    window: Arc<BoxedWindow>,

    passthrough: Option<Mutex<PassThrough>>,
    planner: Mutex<FftPlanner<f32>>,
    samples: Mutex<Vec<f32>>,
    last_samples: Mutex<Option<Vec<f32>>>,
    this: Mutex<Option<Arc<SpectrumAnalyzer>>>,
}

/// Used to pass audio from the input to the output.
/// Useful if you want to hear the audio while analyzing it.
/// Note: The buffers are Vecs of VecDeques because they are storing the samples of each channel individually.
struct PassThrough {
    ctx: InitContext,
    resample_size: usize,
    resampler: SincFixedIn<f32>,
    buffer: Vec<VecDeque<f32>>,
    out_buffer: Vec<VecDeque<f32>>,
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

        let this = Arc::new(Self {
            resolution: 1. / fft_size as f32 * ctx.sample_rate().input as f32,
            ctx,
            fft_size,
            display_range,
            window,

            passthrough,
            planner: Mutex::new(FftPlanner::<f32>::new()),
            samples: Mutex::new(Vec::with_capacity(fft_size)),
            last_samples: Mutex::new(None),
            this: Mutex::new(None),
        });

        this.this.lock().replace(this.clone());
        this
    }

    fn print_row(&self, data: Vec<f32>) {
        // To double the vertical resolution, we use a box drawing character (▀) that is half filled.
        // This means by setting the foreground and background color to different values, we can draw more data on line.
        // So we need to cache one line and when we get the next line, we can draw both.
        // Here we add the new data to this cache if if is not full yet, otherwise we continue.
        let mut last_samples = self.last_samples.lock();
        if last_samples.is_none() {
            *last_samples = Some(data);
            return;
        }

        let mut stdout = BufWriter::new(stdout());
        let console_size = terminal::size().unwrap();
        let bar_width = (console_size.0 as usize / data.len()).max(1);
        let points_per_char = data.len() as f32 / console_size.0 as f32;

        // Calculate the Root Mean Square (RMS) value of the data.
        // This is shown in the top bar
        let mut rms = 0.0;
        let mut n = 0;
        for i in data.iter().chain(last_samples.as_ref().unwrap().iter()) {
            rms += i * i;
            n += 1;
        }
        rms = (rms / n as f32).sqrt();

        // Setup the terminal and print the top line which has some stats
        queue!(
            stdout,
            terminal::ScrollUp(1),
            cursor::MoveTo(0, 0),
            style::Print(self.top_line(console_size, points_per_char, rms)),
            cursor::MoveTo(0, console_size.1.saturating_sub(2)),
        )
        .unwrap();

        // Init some vars for drawing the spectrum line.
        // The way the spectrum is drawn is by figuring out how many FFT bins will need to be put in each char.
        // Then we loop over the data and for each char we average the values of the bins that will be put in that char.
        // Because the number of bins per char is not always an integer, we need to keep track of the error, so we can add it to the next char.
        let mut vals = Vec::new();
        let mut freq_labels = Vec::new();
        let mut error = 0.;
        let mut full_size = 0;

        let prev_data = last_samples.as_ref().unwrap().iter().copied();
        for (i, e) in data.into_iter().zip(prev_data).enumerate() {
            vals.push(e);

            let points = vals.len() as f32 + error;
            if points >= points_per_char {
                error = points - points_per_char;
                freq_labels.push((full_size, self.index_to_freq(i)));

                let bar = HALF_CHAR.repeat(bar_width);
                full_size += bar_width;

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

        // If we don't print a full line, we need to fill the rest with black.
        if console_size.0 as usize > full_size {
            queue!(
                stdout,
                style::SetForegroundColor(COLOR_SCHEME[0].into()),
                style::SetBackgroundColor(COLOR_SCHEME[0].into()),
                style::Print(HALF_CHAR.repeat(console_size.0 as usize - full_size)),
            )
            .unwrap();
        }

        // Prints the frequency labels on the bottom of the screen.
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

    fn handle_events(&self) {
        // Returns if there are no events to process
        let event = event::poll(Duration::ZERO).unwrap();
        if !event {
            return;
        }

        match event::read().unwrap() {
            // Exit if escape is pressed
            event::Event::Key(e) => {
                if e.code == KeyCode::Esc {
                    exit();
                    process::exit(0);
                }
            }
            // Clear the screen if the terminal is resized
            event::Event::Resize(..) => {
                execute!(stdout(), terminal::Clear(terminal::ClearType::All)).unwrap()
            }
            _ => {}
        }
    }

    /// Defines the top status line.
    /// This line contains some stats about the current state of the program:
    /// - FFT size &mdash; The number of samples that are used for each FFT.
    /// - Domain &mdash; The frequency range that is currently displayed.
    /// - BinRes &mdash; The frequency resolution of each FFT bin, derived from the FFT size and the sample rate.
    /// - BinChars &mdash; The number of characters that are used to display each FFT bin, this is a product of the terminal width and the frequency resolution.
    /// - RMS &mdash; The Root Mean Square value of the current FFT data.
    fn top_line(&self, size: (u16, u16), points_per_char: f32, rms: f32) -> String {
        let start = "[RADIO-DATA SPECTRUM ANALYZER]";
        let end = format!(
            "{{FFT size: {}, Window: {}, Domain: {}..{}, Res: {:.1}, RMS: {:.1}}} [ESC: Quit]",
            self.fft_size,
            self.window.name(),
            nice_freq(self.display_range.start as f32),
            nice_freq(self.display_range.end as f32),
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

impl PassThrough {
    /// Creates a new pass-through
    fn new(ctx: InitContext, resample_size: usize) -> Self {
        let channels = ctx.input.channels().min(ctx.output.channels()) as usize;
        let parameters = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        // Inits the resampler
        // This is needed because the input and output sample rates are not always the same.
        // So we have to resample the input to the output sample rate before writing it to the output.
        let resampler = SincFixedIn::new(
            ctx.sample_rate().output as f64 / ctx.sample_rate().input as f64,
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

    /// Adds samples from the input to the buffer.
    /// If the buffer is big enough, it will resample the samples and but them in the output buffer.
    fn add_samples(&mut self, samples: &[f32]) {
        let inp_channels = self.ctx.input.channels() as usize;
        let channels = self.buffer.len();

        // Adds the samples to the buffer of the corresponding channel
        for (i, &e) in samples.iter().enumerate() {
            let channel = i % inp_channels;
            if channel >= channels {
                continue;
            }

            self.buffer[channel].push_back(e);
        }

        // Resamples the samples if the buffer is big enough
        while self.buffer.iter().map(|x| x.len()).max().unwrap_or(0) >= self.resample_size {
            let mut samples = vec![Vec::new(); channels];
            for _ in 0..self.resample_size {
                for (j, e) in samples.iter_mut().enumerate().take(channels) {
                    e.push(self.buffer[j].pop_front().unwrap_or(0.0));
                }
            }

            // dbg!(samples[0].len());
            let out = self.resampler.process(&samples, None).unwrap();
            // dbg!(out[0].len());
            for (i, e) in out.into_iter().enumerate() {
                self.out_buffer[i].extend(e);
            }
        }
    }

    /// Writes the output to the output buffer.
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
        // Prints some info about the current state of the program
        println!("[I] FFT size: {}", self.fft_size);
        println!("[I] Display range: {:?}", self.display_range);
        println!("[I] Resolution: {}", nice_freq(self.resolution));

        // Sets a panic hook
        // This is important because the terminal will be in a weird state if the program panics
        // and you wont be able to close the program.
        panic::set_hook(Box::new(|info| {
            exit();
            eprintln!("{info}");
            process::exit(0)
        }));

        // Enables raw mode and enters the alternate screen
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
                // Converts the samples to complex numbers for the FFT
                let mut buf = Vec::with_capacity(this.fft_size);
                for i in samples.drain(..this.fft_size) {
                    buf.push(Complex::new(i, 0.));
                }

                // Run said FFT
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

                // Handles terminal events like button presses and resizes
                this.handle_events();

                // Call the above function to print the row
                this.print_row(this.window.window(&norm).into_owned());
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

/// Cleans up the terminal and disables raw mode before exiting.
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
