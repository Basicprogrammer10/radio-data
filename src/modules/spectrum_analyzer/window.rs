use std::{collections::VecDeque, f32::consts::E, sync::Arc, time::Instant};

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

use super::{
    egui::{Egui, Gui},
    {color, nice_freq, Renderer, SpectrumAnalyzer},
};
use crate::{misc::ring_buffer::RingBuffer, modules::spectrum_analyzer::Color};

const INIT_SIZE: (u32, u32) = (320, 240);

pub struct WindowRenderer {
    window: Arc<Mutex<Window>>,
}

struct Window {
    analyzer: Arc<SpectrumAnalyzer>,
    new: VecDeque<Vec<f32>>,
    last_frame: Instant,
    frame_history: RingBuffer<f32, 1000>,
    size: (u32, u32),
    resize: bool,
}

impl Renderer for WindowRenderer {
    fn init(&self) {}

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
                    win.size = (size.width, size.height);
                    win.resize = true;
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
            })),
        }
    }
}

impl Window {
    fn draw(&mut self, image: &mut [u8]) {
        let (width, height) = (self.size.0 as usize, self.size.1 as usize);

        if self.resize {
            self.resize = false;
            image.iter_mut().for_each(|x| *x = 0);
        }

        let mut error = 0.0;
        let mut points = Vec::new();
        let mut xi = 0;

        while let Some(row) = self.new.pop_front() {
            let points_per_px = row.len() as f32 / self.size.0 as f32;
            let pxs_per_point = (self.size.0 as usize / row.len()).max(1);

            // scroll everything up one
            let prev = image[(width * 4)..(width * height * 4)].to_owned();
            image[0..(width * (height - 1) * 4)].copy_from_slice(&prev);

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
                        set_pixel(image, width, (xi, height - 1), color);
                        xi += 1;
                    }
                }
            }

            xi = 0;
            error = 0.0;
            points.clear();
        }
    }

    fn top_line(&mut self, ui: &mut Ui) {
        let delta = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();
        self.frame_history.push(delta);

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

impl Gui for Arc<Mutex<Window>> {
    fn ui(&self, ctx: &Context) {
        let mut this = self.lock();
        egui::TopBottomPanel::top("menubar_container").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.label(RichText::new("[RADIO-DATA SPECTRUM ANALYZER]").monospace());
            });
        });

        egui::Window::new("Spectrum Analyzer").show(ctx, |ui| {
            this.top_line(ui);
        });
    }
}

fn set_pixel(image: &mut [u8], row_size: usize, pos: (usize, usize), color: Color) {
    let pixel = (pos.0 + pos.1 * row_size) * 4;
    let color = color.to_slice();
    image[pixel..pixel + 4].copy_from_slice(&color);
}
