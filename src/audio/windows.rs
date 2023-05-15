use std::{borrow::Cow, f32::consts::PI};

pub type BoxedWindow = Box<dyn Window + Send + Sync + 'static>;

pub const WINDOWS: &[&str] = &["square", "hann"];

pub trait Window {
    fn window<'a>(&self, input: &'a [f32]) -> Cow<'a, [f32]>;
}

pub fn get_window(name: &str) -> Option<BoxedWindow> {
    Some(match name.to_ascii_lowercase().as_str() {
        "s" | "square" => Box::new(SquareWindow),
        "h" | "hann" => Box::new(HannWindow),
        _ => return None,
    })
}

pub struct SquareWindow;

impl Window for SquareWindow {
    fn window<'a>(&self, input: &'a [f32]) -> Cow<'a, [f32]> {
        Cow::Borrowed(input)
    }
}

pub struct HannWindow;

impl Window for HannWindow {
    fn window<'a>(&self, input: &'a [f32]) -> Cow<'a, [f32]> {
        let out = input
            .iter()
            .enumerate()
            .map(|(i, &e)| {
                let a = 2.0 * PI * i as f32;
                let n = (a / input.len() as f32).cos();
                let w = 0.5 * (1.0 - n);
                w * e
            })
            .collect();

        Cow::Owned(out)
    }
}
