use clap::ArgMatches;
use cpal::SupportedStreamConfig;

pub mod range_test;

pub trait Module {
    fn name(&self) -> &'static str;
    fn input(&self, _input: &[f32]) {}
    fn output(&self, _output: &mut [f32]) {}
}

pub struct InitContext {
    pub args: ArgMatches,
    pub input: SupportedStreamConfig,
    pub output: SupportedStreamConfig,
}

impl InitContext {
    pub fn output_sr(&self) -> u32 {
        self.output.sample_rate().0
    }
}
