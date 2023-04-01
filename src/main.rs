use std::{f32::consts::PI, fs};

use coding::BinEncoder;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod coding;
mod context;

use context::Context;

const DATA: &[u8] = b"mango";

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .nth(1)
        .expect("no supported config?!")
        .with_sample_rate(cpal::SampleRate(44100));
    let channels = supported_config.channels() as usize;

    println!("Hooked into `{}`", device.name().unwrap());

    // let data = fs::read("/home/connorslade/Downloads/NiceToaster.png").unwrap();
    let mut ctx = Context::new(BinEncoder::new(DATA));

    // let spec = hound::WavSpec {
    //     channels: 1,
    //     sample_rate: 44100,
    //     bits_per_sample: 32,
    //     sample_format: hound::SampleFormat::Float,
    // };
    // let mut writer = hound::WavWriter::create("out.wav", spec).unwrap();

    // for i in ctx.encode {
    //     writer.write_sample(i).unwrap();
    // }

    let stream = device
        .build_output_stream(
            &supported_config.into(),
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut last = 0.0;
                for (i, x) in data.iter_mut().enumerate() {
                    if i % channels == 0 {
                        last = ctx.next().unwrap_or(0.);
                    }

                    *x = last;
                }
            },
            move |err| {
                dbg!(err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();
    std::thread::park();
}
