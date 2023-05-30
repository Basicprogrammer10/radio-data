use std::{collections::VecDeque, f32::consts::E, sync::Arc, time::Instant};

use chrono::Local;
use egui::{Align, Align2, Context, RichText, Slider, Ui};
use egui_extras::{Column, TableBuilder};
use image::{ImageBuffer, Rgba};
use parking_lot::Mutex;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

use super::{
    egui::{Egui, Gui},
    {color, nice_freq, Renderer, SpectrumAnalyzer},
};
use crate::{misc::ring_buffer::RingBuffer, modules::spectrum_analyzer::Color};

const INIT_SIZE: (u32, u32) = (1302, 675);

pub struct WindowRenderer {
    window: Arc<Mutex<Window>>,
}

struct Window {
    /// Reference to the analyzer struct
    analyzer: Arc<SpectrumAnalyzer>,
    /// New ffted data to be drawn
    new: VecDeque<Vec<f32>>,
    /// Last time the frame was drawn
    last_frame: Instant,
    /// History of frame times
    frame_history: RingBuffer<f32, 200>,
    /// Current window size
    size: (u32, u32),

    // == Flags ==
    /// Whether the window was resized since the last frame
    resize: bool,
    /// Wether a image capture should be saved next frame
    capture: bool,
    /// If the info panel should be shown
    show_info: bool,
}

impl Renderer for WindowRenderer {
    fn render(&self, data: Vec<f32>) {
        self.window.lock().new.push_back(data);
    }

    fn block(&self) -> ! {
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let size = LogicalSize::new(INIT_SIZE.0 as f64, INIT_SIZE.1 as f64);
        let window = WindowBuilder::new()
            .with_title("Radio Data - Spectrum Analyzer")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let mut pixels = Pixels::new(INIT_SIZE.0, INIT_SIZE.1, surface_texture).unwrap();
        let mut framework = Egui::new(
            &event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            &pixels,
            self.window.clone(),
        );

        let win = self.window.clone();
        event_loop.run(move |event, _, control_flow| {
            if input.update(&event) {
                if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if let Some(scale_factor) = input.scale_factor() {
                    framework.scale_factor(scale_factor);
                }

                if let Some(size) = input.window_resized() {
                    pixels.resize_buffer(size.width, size.height).unwrap();
                    pixels.resize_surface(size.width, size.height).unwrap();
                    framework.resize(size.width, size.height);
                    let mut win = win.lock();
                    win.resize |= win.size.0 != size.width;
                    win.size = (size.width, size.height);
                    drop(win);
                }

                window.request_redraw();
            }

            match event {
                Event::WindowEvent { event, .. } => {
                    framework.handle_event(&event);
                }
                Event::RedrawRequested(_) => {
                    win.lock().draw(pixels.frame_mut());
                    framework.prepare(&window);

                    pixels
                        .render_with(|encoder, render_target, context| {
                            context.scaling_renderer.render(encoder, render_target);
                            framework.render(encoder, render_target, context);
                            Ok(())
                        })
                        .unwrap();
                }
                _ => (),
            }
        });
    }
}

impl WindowRenderer {
    pub fn new(analyzer: Arc<SpectrumAnalyzer>) -> Self {
        Self {
            window: Arc::new(Mutex::new(Window {
                analyzer,
                new: VecDeque::new(),
                last_frame: Instant::now(),
                frame_history: RingBuffer::new(),
                size: INIT_SIZE,

                resize: false,
                capture: false,
                show_info: true,
            })),
        }
    }
}

impl Window {
    fn draw(&mut self, image: &mut [u8]) {
        let (width, height) = (self.size.0 as usize, self.size.1 as usize);
        let gain = *self.analyzer.gain.read();

        if self.capture {
            self.capture = false;
            let buf =
                ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, image.to_owned())
                    .unwrap();

            let name = format!("capture-{}.png", Local::now().format("%Y-%m-%d-%H-%M-%S"));
            buf.save(&name).unwrap();
            println!("[*] Saving capture to `{}`", name);
        }

        if self.resize {
            self.resize = false;
            image.iter_mut().for_each(|x| *x = 0);
        }

        let mut error = 0.0;
        let mut points = Vec::new();
        let mut xi = 0;

        while let Some(row) = self.new.pop_front() {
            let points_per_px = row.len() as f32 / width as f32;
            let pxs_per_point = (width / row.len()).max(1);

            // scroll everything up one
            let prev = image[(width * 4)..(width * height * 4)].to_owned();
            image[0..(width * (height - 1) * 4)].copy_from_slice(&prev);

            // Draw new row
            for x in row {
                points.push(x * gain);

                let err_points = points.len() as f32 + error;
                if err_points >= points_per_px {
                    error = err_points - points_per_px;

                    let avg = points.iter().copied().sum::<f32>() / points.len() as f32;
                    let val = 1.0 - E.powf(-avg);
                    let color = color(val);

                    for _ in 0..pxs_per_point {
                        set_pixel(image, width, (xi, height - 1), color);
                        xi += 1;
                    }

                    points.clear();
                }
            }

            xi = 0;
            error = 0.0;
        }
    }

    fn top_line(&mut self, ui: &mut Ui) {
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();
        self.frame_history.push(delta);

        // Main info table
        // todo: maybe RMS and FFT resolution
        let analyzer = &self.analyzer;
        let info = [
            ("FPS", format!("{:.2}", self.frame_history.avg().recip())),
            ("FFT size", analyzer.fft_size.to_string()),
            (
                "Sample Rate",
                analyzer.ctx.input.sample_rate().0.to_string(),
            ),
            ("Window", analyzer.window.name().to_owned()),
            (
                "Domain",
                format!(
                    "{}..{}",
                    nice_freq(analyzer.display_range.start as f32),
                    nice_freq(analyzer.display_range.end as f32),
                ),
            ),
        ];

        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::remainder())
            .body(|mut body| {
                for i in info {
                    body.row(15.0, |mut row| {
                        row.col(|col| {
                            col.label(i.0.to_owned() + ":");
                        });
                        row.col(|col| {
                            col.label(i.1);
                        });
                    });
                }
            });
        ui.separator();

        // Gain Control
        let mut gain = *self.analyzer.gain.read();
        ui.add(Slider::new(&mut gain, 0.0..=1.0).text("Gain"));
        *self.analyzer.gain.write() = gain;
        ui.separator();

        // Buttons
        ui.horizontal(|ui| {
            self.resize |= ui.button("Clear").clicked();
            self.capture |= ui.button("Capture").clicked();
        });
    }
}

impl Gui for Arc<Mutex<Window>> {
    fn ui(&self, ctx: &Context) {
        let mut this = self.lock();
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new("[RADIO-DATA SPECTRUM ANALYZER]").monospace());
                ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                    this.show_info ^= ui.button("Menu").clicked();
                });
            });
        });

        if !this.show_info {
            return;
        }

        egui::Window::new("Spectrum Analyzer")
            .anchor(Align2::RIGHT_TOP, [-10.0, 10.0])
            .default_width(50.0)
            .resizable(false)
            .show(ctx, |ui| {
                this.top_line(ui);
            });
    }
}

fn set_pixel(image: &mut [u8], row_size: usize, pos: (usize, usize), color: Color) {
    let pixel = (pos.0 + pos.1 * row_size) * 4;
    let color = color.to_slice();
    image[pixel..pixel + 4].copy_from_slice(&color);
}
