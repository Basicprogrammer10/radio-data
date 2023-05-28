use std::{process, sync::Arc};

use crossbeam::channel;
use macroquad::{
    prelude::{is_key_down, KeyCode, WHITE},
    text::draw_text,
    window::next_frame,
};

use super::{Renderer, SpectrumAnalyzer};
use crate::misc::soon::Soon;

pub struct WindowRenderer {
    pub analyzer: Arc<SpectrumAnalyzer>,
    pub tx: Soon<channel::Sender<Vec<f32>>>,
}

impl Renderer for WindowRenderer {
    fn init(&self) {
        let (tx, rx) = channel::unbounded::<Vec<f32>>();
        macroquad::Window::new("Spectrum Analyzer", amain(rx, self.analyzer.clone()));
        self.tx.replace(tx);
    }

    fn render(&self, data: Vec<f32>) {}
}

async fn amain(rx: channel::Receiver<Vec<f32>>, analyzer: Arc<SpectrumAnalyzer>) {
    loop {
        draw_text(
            &analyzer.top_line((100, 100), 0.5, 0.5),
            0.0,
            50.0,
            25.0,
            WHITE,
        );

        if is_key_down(KeyCode::Escape) {
            process::exit(0);
        }

        next_frame().await;
    }
}
