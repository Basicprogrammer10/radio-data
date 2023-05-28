use std::collections::VecDeque;

use rubato::{InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction};

use crate::modules::InitContext;

/// Used to pass audio from the input to the output.
/// Useful if you want to hear the audio while analyzing it.
/// Note: The buffers are Vecs of VecDeques because they are storing the samples of each channel individually.
pub struct PassThrough {
    ctx: InitContext,
    resample_size: usize,
    resampler: SincFixedIn<f32>,
    buffer: Vec<VecDeque<f32>>,
    out_buffer: Vec<VecDeque<f32>>,
}

impl PassThrough {
    /// Creates a new pass-through
    pub fn new(ctx: InitContext, resample_size: usize) -> Self {
        let channels = ctx.input.channels().min(ctx.output.channels()) as usize;
        let parameters = InterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: InterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        // Inits the resampler
        // This is needed because the input and output sample rates are not always the same.
        // So we have to resample the input to the output sample rate before writing it to the output.
        let resampler = SincFixedIn::new(
            ctx.sample_rate().output as f64 / ctx.sample_rate().input as f64,
            2.,
            parameters,
            resample_size,
            channels,
        )
        .unwrap();

        Self {
            ctx,
            resampler,
            resample_size,
            buffer: vec![VecDeque::new(); channels],
            out_buffer: vec![VecDeque::new(); channels],
        }
    }

    /// Adds samples from the input to the buffer.
    /// If the buffer is big enough, it will resample the samples and but them in the output buffer.
    pub fn add_samples(&mut self, samples: &[f32]) {
        let inp_channels = self.ctx.input.channels() as usize;
        let channels = self.buffer.len();

        // Adds the samples to the buffer of the corresponding channel
        for (i, &e) in samples.iter().enumerate() {
            let channel = i % inp_channels;
            if channel >= channels {
                continue;
            }

            self.buffer[channel].push_back(e);
        }

        // Resamples the samples if the buffer is big enough
        while self.buffer.iter().map(|x| x.len()).max().unwrap_or(0) >= self.resample_size {
            let mut samples = vec![Vec::new(); channels];
            for _ in 0..self.resample_size {
                for (j, e) in samples.iter_mut().enumerate().take(channels) {
                    e.push(self.buffer[j].pop_front().unwrap_or(0.0));
                }
            }

            // dbg!(samples[0].len());
            let out = self.resampler.process(&samples, None).unwrap();
            // dbg!(out[0].len());
            for (i, e) in out.into_iter().enumerate() {
                self.out_buffer[i].extend(e);
            }
        }
    }

    /// Writes the output to the output buffer.
    pub fn write_output(&mut self, output: &mut [f32]) {
        let out_channels = self.ctx.output.channels() as usize;

        for (i, e) in output.iter_mut().enumerate() {
            let channel = i % self.ctx.output.channels() as usize;
            if channel >= out_channels {
                *e = 0.0;
                continue;
            }

            *e = self.out_buffer[channel].pop_front().unwrap_or(0.0);
        }
    }
}