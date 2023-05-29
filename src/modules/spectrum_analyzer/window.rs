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

use super::egui::{Egui, Gui};
use super::{nice_freq, Renderer, SpectrumAnalyzer};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

pub struct WindowRenderer {
    window: Arc<Window>,
}

struct Window {
    analyzer: Arc<SpectrumAnalyzer>,
    history: RwLock<Vec<Vec<f32>>>,
    last_frame: Mutex<f64>,
}

impl Renderer for WindowRenderer {
    fn init(&self) {}

    fn render(&self, data: Vec<f32>) {
        self.window.history.write().push(data);
    }

    fn block(&self) -> ! {
        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let window = {
            let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
            WindowBuilder::new()
                .with_title("Radio Data - Spectrum Analyzer")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)
                .unwrap()
        };

        let (mut pixels, mut framework) = {
            let window_size = window.inner_size();
            let scale_factor = window.scale_factor() as f32;
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();
            let framework = Egui::new(
                &event_loop,
                window_size.width,
                window_size.height,
                scale_factor,
                &pixels,
                self.window.clone(),
            );

            (pixels, framework)
        };

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
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("[pixels.resize_surface] {err}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    framework.resize(size.width, size.height);
                }

                // world.update();
                window.request_redraw();
            }

            match event {
                Event::WindowEvent { event, .. } => {
                    // Update egui inputs
                    framework.handle_event(&event);
                }
                Event::RedrawRequested(_) => {
                    win.draw(pixels.frame_mut());
                    framework.prepare(&window);

                    let render_result = pixels.render_with(|encoder, render_target, context| {
                        context.scaling_renderer.render(encoder, render_target);
                        framework.render(encoder, render_target, context);
                        Ok(())
                    });

                    if let Err(err) = render_result {
                        eprintln!("[pixels.render] {err}");
                        *control_flow = ControlFlow::Exit;
                    }
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
            }),
        }
    }
}

impl Window {
    fn draw(&self, image: &mut [u8]) {}

    fn top_line(&self, ui: &mut Ui) {
        let now = now();
        let mut last_frame = self.last_frame.lock();
        let delta = now - *last_frame;
        *last_frame = now;

        let analyzer = &self.analyzer;
        let info = [
            ("FPS", format!("{:.2}", delta.recip())),
            ("FFT size", analyzer.fft_size.to_string()),
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
