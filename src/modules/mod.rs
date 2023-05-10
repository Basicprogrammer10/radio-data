use clap::ArgMatches;
use cpal::SupportedStreamConfig;

use crate::misc::SampleRate;

pub mod dtmf_receive;
pub mod dtmf_send;
pub mod range_test;
pub mod spectrum_analyzer;

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
    pub fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.input.sample_rate().0, self.output.sample_rate().0)
    }
}
