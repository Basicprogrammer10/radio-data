use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod coding;
mod context;
mod tone;

use coding::dtmf::{DtmfDecoder, DtmfEncoder};
use rodio::{source::SineWave, OutputStream, Sink, Source};

use crate::coding::dtmf;

const DATA: &[u8] = b"I like fried rice!sp";
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

    let data = dtmf::bin_to_dtmf(DATA);
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
            move |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    let mut out = Vec::new();
    let mut skip = false;
    let mut decode = DtmfDecoder::new(move |chr| {
        if skip {
            skip = false;
            println!("[*] Skip {chr}");
            return;
        }
        println!("[*] Received Code: {chr}");
        out.push(chr as u8);

        if out.len() >= 4 && &out[out.len() - 4..] == b"1234" {
            println!("GOT CODE");
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            // Add a dummy source of the sake of the example.
            let source = SineWave::new(440.0).take_duration(Duration::from_secs_f32(4.));
            sink.append(source);
            sink.play();
            sink.sleep_until_end();
            out.clear();
        }

        let size = out.len();
        if size > 2 && out[size - 2] == b'1' && out[size - 1] == b'#' {
            let text = dtmf::dtmf_to_bin(&out);
            let text = text.iter().map(|x| *x as char).collect::<String>();
            println!("{:?}", &text[0..text.len() - 2]);
            out.clear();
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
            |err| eprintln!("[-] Error: {err:?}"),
            None,
        )
        .unwrap();

    stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
