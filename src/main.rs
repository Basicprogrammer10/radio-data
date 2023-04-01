use std::{fs::File, io::BufReader, sync::Arc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::OutputStream;

mod coding;
mod context;
mod tone;

use coding::dtmf_decode::{DtmfDecoder, DtmfEncoder};

use crate::coding::dtmf_decode;

const DATA: &[u8] = b"Save the turtles!sp";
const SAMPLE_RATE: u32 = 44100;

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
        .with_sample_rate(cpal::SampleRate(SAMPLE_RATE));
    let channels = supported_config.channels() as usize;

    let input_device = host
        .default_input_device()
        .expect("no input device available");
    let input_supported_config = input_device
        .default_input_config()
        .expect("error while querying configs");
    let input_channels = input_supported_config.channels() as usize;

    println!("[*] Hooked into `{}`", device.name().unwrap());

    let data = dtmf_decode::bin_to_dtmf(DATA);
    println!("{:?}", data.iter().map(|x| *x as char).collect::<Vec<_>>());
    let mut dtmf = DtmfEncoder::new(&data);

    let stream = device
        .build_output_stream(
            &supported_config.into(),
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                let mut last = 0.0;
                for (i, x) in data.iter_mut().enumerate() {
                    if i % channels == 0 {
                        // last = ctx.next().unwrap_or(0.);
                        last = dtmf.next().unwrap_or(0.0);
                    }

                    *x = last;
                }
            },
            move |err| eprintln!("[-] Error: {}", err),
            None,
        )
        .unwrap();

    let mut out = Vec::new();
    let mut skip = true;
    let mut decode = DtmfDecoder::new(move |chr| {
        if skip {
            skip = false;
            println!("[*] Skip {}", chr);
            return;
        }
        println!("[*] Receded Code: {}", chr);
        out.push(chr as u8);

        let size = out.len();
        if size > 2 && out[size - 2] == b'1' && out[size - 1] == b'#' {
            let out = dtmf_decode::dtmf_to_bin(&out);
            println!("{:?}", out.iter().map(|x| *x as char).collect::<String>());
        }
    });

    let input_stream = device
        .build_input_stream(
            &input_supported_config.into(),
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                let mut work = Vec::new();
                for (i, x) in data.iter().enumerate() {
                    if i % input_channels == 0 {
                        // decode.add(*x);
                        work.push(*x);
                    }
                }

                decode.process(&work);
            },
            |err| eprintln!("[-] Error: {:?}", err),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
