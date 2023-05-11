#[derive(Debug, Clone, Copy)]
pub struct SampleRate {
    pub input: u32,
    pub output: u32,
}

impl SampleRate {
    pub fn new(input: u32, output: u32) -> Self {
        Self { input, output }
    }

    pub fn from_hz(hz: u32) -> Self {
        Self::new(hz, hz)
    }
}

impl From<u32> for SampleRate {
    fn from(hz: u32) -> Self {
        Self::from_hz(hz)
    }
}
