use std::sync::Arc;

use parking_lot::Mutex;
use rustfft::{num_complex::Complex, Fft, FftPlanner};

use super::{InitContext, Module};

pub struct SpectrumAnalyzer {
    // fft: Arc<dyn Fft<f32>>,
    planner: Mutex<FftPlanner<f32>>,
}

impl SpectrumAnalyzer {
    pub fn new(ctx: InitContext) -> Arc<Self> {
        let mut planner = FftPlanner::<f32>::new();

        Arc::new(Self {
            planner: Mutex::new(planner),
        })
    }
}

impl Module for SpectrumAnalyzer {
    fn name(&self) -> &'static str {
        "spectrum_analyzer"
    }

    fn input(&self, input: &[f32]) {
        let mut buf = Vec::with_capacity(input.len());
        for i in input {
            buf.push(Complex::new(*i, 0.));
        }

        let fft = self.planner.lock().plan_fft_forward(input.len());
        fft.process(&mut buf);
        dbg!(buf);
    }
}

// https://github.com/phip1611/spectrum-analyzer/blob/main/README.md
