use std::{process, sync::Arc, thread, f32::consts::E};

use crossbeam::channel;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use macroquad::{
    color,
    miniquad::Texture,
    prelude::{
        is_key_down, is_quit_requested, prevent_quit, scene::clear, Color, KeyCode, BLACK, BLUE,
        RED, WHITE,
    },
    text::draw_text,
    texture::{draw_texture, Image, Texture2D},
    time::get_fps,
    window::{clear_background, next_frame, screen_height, screen_width},
};
use parking_lot::{Mutex, RwLock};

use super::{nice_freq, Renderer, SpectrumAnalyzer, get_color, color};

pub struct WindowRenderer {
    analyzer: Arc<SpectrumAnalyzer>,
    history: Arc<RwLock<Vec<Vec<f32>>>>,
}

impl Renderer for WindowRenderer {
    fn init(&self) {
        let analyzer = self.analyzer.clone();
        let history = self.history.clone();
        thread::spawn(|| macroquad::Window::new("Spectrum Analyzer", amain(analyzer, history)));
    }

    fn render(&self, data: Vec<f32>) {
        self.history.write().push(data);
    }
}

impl WindowRenderer {
    pub fn new(analyzer: Arc<SpectrumAnalyzer>) -> Self {
        Self {
            analyzer,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

async fn amain(analyzer: Arc<SpectrumAnalyzer>, history: Arc<RwLock<Vec<Vec<f32>>>>) {
    prevent_quit();

    loop {
        if is_key_down(KeyCode::Escape) || is_quit_requested() {
            process::exit(0);
        }

        let ui_analyzer = analyzer.clone();
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Spectrum Analyzer").show(egui_ctx, |ui| {
                ui.scope(|ui| top_line(ui_analyzer, ui));
            });
        });

        clear_background(color::BLACK);

        let size = (screen_width() as u16, screen_height() as u16);
        let mut image = Image::gen_image_color(size.0, size.1, RED);

        for (row_index, row) in history.read().iter().enumerate() {
            for x in 0..size.0 {
                let val = *row.get(x as usize).unwrap_or(&0.0);
                let color = color(1. - E.powf(-val));
                image.set_pixel(
                    x as u32,
                    row_index as u32,
                    color.into(),
                )
            }
        }

        let texture = Texture2D::from_image(&image);
        texture.update(&image);
        draw_texture(texture, 0.0, 0.0, WHITE);

        egui_macroquad::draw();
        next_frame().await;
    }
}

fn top_line(analyzer: Arc<SpectrumAnalyzer>, ui: &mut Ui) {
    let info = [
        ("FPS", get_fps().to_string()),
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
