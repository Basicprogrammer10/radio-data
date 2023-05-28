use std::{process, sync::Arc};

use crossbeam::channel;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use macroquad::{
    color,
    prelude::{is_key_down, scene::clear, KeyCode, WHITE},
    text::draw_text,
    window::{clear_background, next_frame},
};
use parking_lot::Mutex;

use super::{nice_freq, Renderer, SpectrumAnalyzer};

pub struct WindowRenderer {
    pub analyzer: Arc<SpectrumAnalyzer>,
    history: Mutex<Vec<Vec<f32>>>,
}

impl Renderer for WindowRenderer {
    fn init(&self) {
        macroquad::Window::new("Spectrum Analyzer", amain(self.analyzer.clone()));
    }

    fn render(&self, data: Vec<f32>) {}
}

impl WindowRenderer {
    pub fn new(analyzer: Arc<SpectrumAnalyzer>) -> Self {
        Self {
            analyzer,
            history: Mutex::new(Vec::new()),
        }
    }
}

async fn amain(analyzer: Arc<SpectrumAnalyzer>) {
    loop {
        if is_key_down(KeyCode::Escape) {
            process::exit(0);
        }

        let ui_analyzer = analyzer.clone();
        egui_macroquad::ui(|egui_ctx| {
            egui::Window::new("Spectrum Analyzer").show(egui_ctx, |ui| {
                ui.scope(|ui| top_line(ui_analyzer, ui));
            });
        });

        clear_background(color::BLACK);
        egui_macroquad::draw();
        next_frame().await;
    }
}

fn top_line(analyzer: Arc<SpectrumAnalyzer>, ui: &mut Ui) {
    let info = [
        ("FFT size", analyzer.fft_size.to_string()),
        ("Window", analyzer.window.name().to_owned()),
        (
            "Domain",
            format!(
                "{} to {}",
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
