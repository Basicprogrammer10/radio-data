// todo: documentation

use std::{borrow::Cow, f32::consts::PI};

pub type BoxedWindow = Box<dyn Window + Send + Sync + 'static>;

pub const WINDOWS: &[&str] = &["square", "hann", "blackman"];

pub trait Window {
    fn name(&self) -> &'static str;
    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]>;
}

pub fn get_window(name: &str) -> Option<BoxedWindow> {
    Some(match name.to_ascii_lowercase().as_str() {
        "s" | "square" => Box::new(SquareWindow),
        "h" | "hann" => Box::new(HannWindow),
        "b" | "blackman" => Box::new(BlackmanNuttallWindow),
        _ => return None,
    })
}

pub struct SquareWindow;

impl Window for SquareWindow {
    fn name(&self) -> &'static str {
        "square"
    }

    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]> {
        Cow::Borrowed(samples)
    }
}

pub struct HannWindow;

impl Window for HannWindow {
    fn name(&self) -> &'static str {
        "hann"
    }

    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]> {
        let out = samples
            .iter()
            .enumerate()
            .map(|(i, &e)| {
                let a = (2.0 * PI * i as f32) / samples.len() as f32;
                let w = 0.5 * (1.0 - a.cos());
                w * e
            })
            .collect();

        Cow::Owned(out)
    }
}

pub struct BlackmanNuttallWindow;

impl Window for BlackmanNuttallWindow {
    fn name(&self) -> &'static str {
        "blackman"
    }

    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]> {
        const A0: f32 = 0.3635819;
        const A1: f32 = 0.4891775;
        const A2: f32 = 0.1365995;
        const A3: f32 = 0.0106411;

        let n = samples.len() as f32;
        let out = samples
            .iter()
            .enumerate()
            .map(|(i, &e)| {
                let c1 = (2.0 * PI * i as f32) / n;
                let c2 = (4.0 * PI * i as f32) / n;
                let c3 = (6.0 * PI * i as f32) / n;
                let w = A0 - A1 * c1.cos() + A2 * c2.cos() - A3 * c3.cos();
                e * w
            })
            .collect();

        Cow::Owned(out)
    }
}
