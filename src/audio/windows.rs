//! Windowing functions ([wikipedia](https://en.wikipedia.org/wiki/Windowing_functions))

use std::{borrow::Cow, f32::consts::PI};

/// A boxed, thread safe Window trait object
pub type BoxedWindow = Box<dyn Window + Send + Sync + 'static>;

/// Different valid windows.
/// This is only used in the command like error message
pub const WINDOWS: &[&str] = &["square", "hann", "blackman"];

/// Trait implemented by window functions.
/// Takes in a slice of samples and outputs those same samples after being transformed
pub trait Window {
    /// Get the name of the window function.
    /// Used in the info bat of the spectrum analyzer
    fn name(&self) -> &'static str;
    /// The main method to run the windowing function
    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]>;
}

/// Gets a windowing function by its name.
/// Returns None if there is not one named `name`.
/// Used in command like arg parsing
pub fn get_window(name: &str) -> Option<BoxedWindow> {
    Some(match name.to_ascii_lowercase().as_str() {
        "s" | "square" => Box::new(SquareWindow),
        "h" | "hann" => Box::new(HannWindow),
        "b" | "blackman" => Box::new(BlackmanNuttallWindow),
        _ => return None,
    })
}

/// Basically does nothing.
/// \[[Rectangular Window](https://en.wikipedia.org/wiki/Window_function#Rectangular_window)\]
pub struct SquareWindow;

impl Window for SquareWindow {
    fn name(&self) -> &'static str {
        "square"
    }

    fn window<'a>(&self, samples: &'a [f32]) -> Cow<'a, [f32]> {
        Cow::Borrowed(samples)
    }
}

/// Hann windowing function.
/// \[[Hann and Hamming Windows](https://en.wikipedia.org/wiki/Window_function#Hann_and_Hamming_windows)\]
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

/// Blackman Nuttall windowing function.
/// \[[Blackman Nuttall Window](https://en.wikipedia.org/wiki/Window_function#Blackman%E2%80%93Nuttall_window)\]
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
