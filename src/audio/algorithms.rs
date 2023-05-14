//! Audio processing algorithms.

use std::f32::consts::PI;

use num_complex::Complex;

/// Implements the [Goertzel algorithm](https://en.wikipedia.org/wiki/Goertzel_algorithm) to find the magnitude of a frequency in a slice of samples.
pub fn goertzel_mag(freq: f32, samples: &[f32], sample_rate: u32) -> f32 {
    let k = (0.5 + (samples.len() as f32 * freq) / sample_rate as f32).floor();
    let omega = (2.0 * PI * k) / samples.len() as f32;
    let sin = omega.sin();
    let cos = omega.cos();
    let coeff = cos * 2.0;

    let mut s1 = 0.0;
    let mut s2 = 0.0;

    for i in samples {
        let s = coeff * s1 - s2 + i;
        s2 = s1;
        s1 = s;
    }

    let real = s1 - s2 * cos;
    let imag = s2 * sin;

    Complex::new(real, imag).norm()
}
