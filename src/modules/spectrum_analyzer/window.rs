use std::collections::VecDeque;
use std::f32::consts::E;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use egui::{Context, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use parking_lot::Mutex;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

use crate::misc::ring_buffer::RingBuffer;
use crate::modules::spectrum_analyzer::Color;

use super::egui::{Egui, Gui};
use super::{color, nice_freq, Renderer, SpectrumAnalyzer};

const INIT_SIZE: (u32, u32) = (320, 240);

pub struct WindowRenderer {
    window: Arc<Window>,
}

struct Window {
    analyzer: Arc<SpectrumAnalyzer>,
    new: Mutex<VecDeque<Vec<f32>>>,
    last_frame: Mutex<Instant>,
    frame_history: Mutex<RingBuffer<f32, 1000>>,
    size: (AtomicU32, AtomicU32),
}

impl Renderer for WindowRenderer {
    fn init(&self) {}

    fn render(&self, data: Vec<f32>) {
        self.window.new.lock().push_back(data);
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
                    win.size.0.store(size.width, Ordering::Relaxed);
                    win.size.1.store(size.height, Ordering::Relaxed);
                    framework.resize(size.width, size.height);
                }

                window.request_redraw();
            }

            match event {
                Event::WindowEvent { event, .. } => {
                    framework.handle_event(&event);
                }
                Event::RedrawRequested(_) => {
                    win.draw(pixels.frame_mut());
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
            window: Arc::new(Window {
                analyzer,
                new: Mutex::new(VecDeque::new()),
                last_frame: Mutex::new(Instant::now()),
                frame_history: Mutex::new(RingBuffer::new()),
                size: (AtomicU32::new(INIT_SIZE.0), AtomicU32::new(INIT_SIZE.1)),
            }),
        }
    }
}

impl Window {
    fn draw(&self, image: &mut [u8]) {
        let row_size = self.size.0.load(Ordering::Relaxed) as usize;
        let rows = self.size.1.load(Ordering::Relaxed) as usize;
        let mut new = self.new.lock(); // todo: make mutex
        let image_len = image.len();

        let mut error = 0.0;
        let mut points = Vec::new();
        let mut xi = 0;

        while let Some(row) = new.pop_front() {
            let points_per_px = row.len() as f32 / row_size as f32;
            let pxs_per_point = (row_size / row.len()).max(1);

            // scroll everything up one
            let prev = image[(row_size * 4)..(row_size * rows * 4)].to_owned();
            debug_assert_eq!(prev.len(), row_size * (rows - 1) * 4);
            debug_assert_eq!(image_len, row_size * rows * 4);
            image[0..(row_size * (rows - 1) * 4)].copy_from_slice(&prev);

            // Draw new row
            for x in row {
                points.push(x);

                let err_points = points.len() as f32 + error;
                if err_points >= points_per_px {
                    error = err_points - points_per_px;

                    let avg = points.iter().copied().sum::<f32>() / points.len() as f32;
                    let val = 1.0 - E.powf(-avg);
                    let color = color(val);

                    for _ in 0..pxs_per_point {
                        set_pixel(image, row_size, (xi, rows - 1), color);
                        xi += 1;
                    }
                }
            }

            xi = 0;
            error = 0.0;
            points.clear();
        }
    }

    fn top_line(&self, ui: &mut Ui) {
        let mut last_frame = self.last_frame.lock();
        let delta = last_frame.elapsed().as_secs_f32();
        *last_frame = Instant::now();
        let mut history = self.frame_history.lock();
        history.push(delta);

        let analyzer = &self.analyzer;
        let info = [
            ("FPS", format!("{:.2}", history.avg().recip())),
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
            ("Gain", analyzer.gain.to_string()),
            ("Res", "".to_string()),
            ("Rms", "".to_string()),
        ];

        TableBuilder::new(ui)
            .column(Column::auto())
            .column(Column::auto())
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
    }
}

impl Gui for Arc<Window> {
    fn ui(&self, ctx: &Context) {
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new("[RADIO-DATA SPECTRUM ANALYZER]").monospace());
            });
        });

        egui::Window::new("Spectrum Analyzer").show(ctx, |ui| {
            self.top_line(ui);
        });
    }
}

fn set_pixel(image: &mut [u8], row_size: usize, pos: (usize, usize), color: Color) {
    let pixel = (pos.0 + pos.1 * row_size) * 4;
    let color = color.to_slice();
    image[pixel..pixel + 4].copy_from_slice(&color);
}
