use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod coding;
mod context;
mod misc;
mod modules;
mod tone;

const SAMPLE_RATE: u32 = 44100;

fn main() {
    let module = modules::modules()[0].clone();
    println!("[*] Running module `{}`", module.name());

    // Setup audio devices
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");
    let supported_config = device
        .default_output_config()
        .expect("no supported config?!");

    let input_device = host
        .default_input_device()
        .expect("no input device available");
    let input_supported_config = input_device
        .default_input_config()
        .expect("error while querying configs");

    println!(
        "[*] Input  hooked into `{}` ({})",
        input_device.name().unwrap(),
        input_supported_config.sample_rate().0
    );
    println!(
        "[*] Output hooked into `{}` ({})",
        device.name().unwrap(),
        supported_config.sample_rate().0
    );

    let output_stream = {
        let module = module.clone();
        let output_config = Arc::new(supported_config.clone());
        device
            .build_output_stream(
                &supported_config.into(),
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    module.output(data, output_config.clone());
                },
                move |err| eprintln!("[-] Error: {err}"),
                None,
            )
            .unwrap()
    };

    let input_stream = {
        let input_cfg = Arc::new(input_supported_config.clone());
        device
            .build_input_stream(
                &input_supported_config.into(),
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    module.input(data, input_cfg.clone());
                },
                |err| eprintln!("[-] Error: {err:?}"),
                None,
            )
            .unwrap()
    };

    output_stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
