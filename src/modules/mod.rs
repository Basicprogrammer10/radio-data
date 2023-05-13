use std::borrow::Cow;

use clap::ArgMatches;
use cpal::{InputCallbackInfo, OutputCallbackInfo, SupportedStreamConfig};

use crate::misc::SampleRate;

pub mod dtmf_receive;
pub mod dtmf_send;
pub mod morse_code;
pub mod range_test;
pub mod spectrum_analyzer;
pub mod true_random;

pub trait Module {
    fn name(&self) -> &'static str;
    fn init(&self) {}
    fn input(&self, _input: &[f32]) {}
    fn output(&self, _output: &mut [f32]) {}

    fn input_raw(&self, input: &[f32], _info: &InputCallbackInfo, gain: f32) {
        let input = match gain {
            i if i == 1.0 => Cow::Borrowed(input),
            _ => Cow::Owned(input.iter().map(|&x| x * gain).collect()),
        };
        self.input(&input);
    }

    fn output_raw(&self, output: &mut [f32], _info: &OutputCallbackInfo, gain: f32) {
        self.output(output);

        if gain != 1.0 {
            for x in output {
                *x *= gain;
            }
        }
    }
}

#[derive(Clone)]
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
