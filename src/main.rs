use std::{fs::File, io::BufReader, thread, sync::Arc};

use coding::BinEncoder;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod coding;
mod context;
mod tone;

use context::Context;
use rodio::{Decoder, OutputStream, Source};

use crate::coding::{dtmf_decode::DtmfDecoder, BinDecoder};

// const DATA: &[u8] = include_bytes!("../bee_movie.txt");
const DATA: &[u8] = b"eggs fresh fresh eggs!";
const SAMPLE_RATE: u32 = 44100;
// const PACKET_LENGTH: u32 = 16;
// const PACKET_SLEEP: f32 = SAMPLE_RATE as f32 * 0.75;

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

    // let data = fs::read("/home/connorslade/Downloads/NiceToaster.png").unwrap();
    let mut ctx = Context::new(BinEncoder::new(DATA));

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let file = BufReader::new(File::open("./egg.mp3").unwrap());
    // let source = Decoder::new(file).unwrap();
    let audio = Arc::new(stream_handle.play_once(file).unwrap());
    audio.pause();

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
            move |err| eprintln!("[-] Error: {}", err),
            None,
        )
        .unwrap();

    let mut decode = DtmfDecoder::new(move |chr| {
        println!("[*] Receded Code: {}", chr);

        if chr == 'A' {
            println!(" | Play Pause");
            if audio.is_paused() {
                audio.play();
            } else {
                audio.pause();
            }
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

    // stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
