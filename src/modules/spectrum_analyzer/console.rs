use std::{
    io::{stdout, Write},
    panic, process,
    sync::Arc,
    thread,
    time::Duration,
};

use crate::{
    misc::{buf_writer::BufWriter, soon::Soon},
    modules::spectrum_analyzer::{get_color, COLOR_SCHEME},
};
use crossbeam::channel::{self, Sender};
use crossterm::{
    cursor,
    event::{self, KeyCode},
    execute, queue, style, terminal,
};
use parking_lot::Mutex;

use super::{nice_freq, Renderer, SpectrumAnalyzer};

const HALF_CHAR: &str = "▀";

pub struct ConsoleRenderer {
    analyzer: Arc<SpectrumAnalyzer>,
    last_samples: Mutex<Option<Vec<f32>>>,
    render_thread: Soon<Sender<Vec<f32>>>,
}

impl Renderer for ConsoleRenderer {
    fn init(&self) {
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

    fn render(&self, data: Vec<f32>) {
        self.print_row(data);
        self.handle_events();
    }
}

impl ConsoleRenderer {
    pub fn new(analyzer: Arc<SpectrumAnalyzer>) -> Arc<Self> {
        let this = Arc::new(Self {
            analyzer,
            last_samples: Mutex::new(None),
            render_thread: Soon::empty(),
        });

        let this_ref = this.clone();
        let (tx, rx) = channel::unbounded();
        thread::spawn(move || {
            for i in rx {
                this_ref.print_row(i)
            }
        });
        this.render_thread.replace(tx);

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

        // todo: maybe remove BufWriter
        let mut stdout = BufWriter::new(stdout());
        let console_size = terminal::size().unwrap();
        let points_per_char = data.len() as f32 / console_size.0 as f32;
        let bar_width = points_per_char.recip();
        let gain = *self.analyzer.gain.read();

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
        let mut point_error = 0.0;
        let mut char_error = 0.0;
        let mut full_size = 0;

        let prev_data = last_samples.as_ref().unwrap().iter().copied();
        for (i, e) in data.into_iter().zip(prev_data).enumerate() {
            vals.push((e.0 * gain, e.1 * gain));
            point_error += 1.0;

            if point_error >= points_per_char {
                freq_labels.push((full_size, self.analyzer.index_to_freq(i)));
                char_error += bar_width;
                point_error -= 1.0;

                let width = (char_error / bar_width) as usize;
                let bar = HALF_CHAR.repeat(width);
                char_error -= width as f32;
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

        // // If we don't print a full line, we need to fill the rest with black.
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
            i += ((freq.len() as f32 + 3.0) / bar_width) as usize;

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
    /// - Gain &mdash; The gain that is applied to the data when displaying.
    /// - Res &mdash; The frequency resolution of each character used to display the spectrum.
    /// - RMS &mdash; The Root Mean Square value of the current FFT data.
    fn top_line(&self, size: (u16, u16), points_per_char: f32, rms: f32) -> String {
        let start = "[RADIO-DATA SPECTRUM ANALYZER]";
        let end = format!(
            "{{FFT size: {}, Window: {}, Domain: {}..{}, Gain: {:.1}, Res: {}, RMS: {:.1}}} [ESC: Quit]",
            self.analyzer.fft_size,
            self.analyzer.window.name(),
            nice_freq(self.analyzer.display_range.start as f32),
            nice_freq(self.analyzer.display_range.end as f32),
            self.analyzer.gain.read(),
            nice_freq(self.analyzer.resolution * points_per_char),
            rms
        );

        let diff = (size.0 as usize).saturating_sub(start.len() + end.len());
        format!("{}{}{}", start, " ".repeat(diff), end)
    }
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
