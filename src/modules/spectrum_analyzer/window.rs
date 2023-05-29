use std::f32::consts::E;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use egui::{Context, RichText, Ui};
use egui_extras::{Column, TableBuilder};
use parking_lot::{Mutex, RwLock};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

use crate::modules::spectrum_analyzer::Color;

use super::egui::{Egui, Gui};
use super::{color, nice_freq, Renderer, SpectrumAnalyzer};

const INIT_SIZE: (u32, u32) = (320, 240);

pub struct WindowRenderer {
    window: Arc<Window>,
}

struct Window {
    analyzer: Arc<SpectrumAnalyzer>,
    history: RwLock<Vec<Vec<f32>>>,
    last_frame: Mutex<f64>,
    size: (AtomicU32, AtomicU32),
}

impl Renderer for WindowRenderer {
    fn init(&self) {}

    fn render(&self, data: Vec<f32>) {
        self.window.history.write().push(data);
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
                history: RwLock::new(Vec::new()),
                last_frame: Mutex::new(now()),
                size: (AtomicU32::new(INIT_SIZE.0), AtomicU32::new(INIT_SIZE.1)),
            }),
        }
    }
}

impl Window {
    fn draw(&self, image: &mut [u8]) {
        let row_size = self.size.0.load(Ordering::Relaxed) as usize;
        let rows = self.size.1.load(Ordering::Relaxed) as usize;
        let history = self.history.read();

        let show_rows = rows.min(history.len());
        for ri in 0..show_rows {
            let history_index = history.len() - ri - 1;
            let row = history.get(history_index).unwrap();

            let ri = rows - ri - 1;
            for (xi, x) in row.iter().enumerate() {
                let val = 1. - E.powf(-x);
                let color = color(val);
                set_pixel(image, row_size, (xi, ri), color);
            }
        }
    }

    fn top_line(&self, ui: &mut Ui) {
        let now = now();
        let mut last_frame = self.last_frame.lock();
        let delta = now - *last_frame;
        *last_frame = now;

        let analyzer = &self.analyzer;
        let info = [
            ("FPS", format!("{:.2}", delta.recip())),
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

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn set_pixel(image: &mut [u8], row_size: usize, pos: (usize, usize), color: Color) {
    let pixel = (pos.0 + pos.1 * row_size) * 4;
    let color = color.to_slice();
    image[pixel..pixel + 4].copy_from_slice(&color);
}
