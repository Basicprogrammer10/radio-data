//! The different modules (subcommands) that can be used in the program.

use std::borrow::Cow;

use clap::ArgMatches;
use cpal::{InputCallbackInfo, OutputCallbackInfo, SupportedStreamConfig};

use crate::misc::SampleRate;

pub mod dtmf;
pub mod morse;
pub mod range_test;
pub mod spectrum_analyzer;
pub mod true_random;

/// The trait implemented by all modules that allows handling audio input and output.
pub trait Module {
    /// Gets the module's name.
    fn name(&self) -> &'static str;
    /// Called when the module is initialized.
    fn init(&self) {}
    /// Input callback.
    /// The different channels are interleaved, so if there are two channels the format will be `[L, R, L, R, ...]`.
    /// Note: If a input gain is set, the input will be multiplied by that gain before being passed to this function.
    fn input(&self, _input: &[f32]) {}
    /// Output callback.
    /// The different channels are interleaved, so if there are two channels the format will be `[L, R, L, R, ...]`.
    /// Note: If a output gain is set, the output will be multiplied by that gain after being passed to this function.
    fn output(&self, _output: &mut [f32]) {}

    /// Raw input callback.
    /// This takes in the raw input data, without any gain applied.
    /// Will call `self.input` by default.
    fn input_raw(&self, input: &[f32], _info: &InputCallbackInfo, gain: f32) {
        let input = match gain {
            i if i == 1.0 => Cow::Borrowed(input),
            _ => Cow::Owned(input.iter().map(|&x| x * gain).collect()),
        };
        self.input(&input);
    }

    /// Raw output callback.
    /// This sets the raw output data.
    /// Will call `self.output` and apply the output gain by default.
    fn output_raw(&self, output: &mut [f32], _info: &OutputCallbackInfo, gain: f32) {
        self.output(output);

        if gain != 1.0 {
            for x in output {
                *x *= gain;
            }
        }
    }
}

/// The context passed to the module when it is initialized.
/// Used to get the command line arguments and the input and output sample rates.
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
