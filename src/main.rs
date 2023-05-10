use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod args;
mod audio;
mod coding;
mod consts;
mod misc;
mod modules;

fn main() {
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

    let module = args::parse_args(supported_config.clone(), input_supported_config.clone());
    println!("[*] Running module `{}`", module.name());

    println!(
        "[*] Input  hooked into `{}` ({})",
        input_device.name().unwrap(),
        supported_config.sample_rate().0
    );
    println!(
        "[*] Output hooked into `{}` ({})",
        device.name().unwrap(),
        input_supported_config.sample_rate().0
    );

    module.init();
    let module_ref = module.clone();
    let output_stream = device
        .build_output_stream(
            &supported_config.into(),
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| module_ref.output(data),
            move |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    let input_stream = input_device
        .build_input_stream(
            &input_supported_config.into(),
            move |data: &[f32], _info: &cpal::InputCallbackInfo| module.input(data),
            |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    output_stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
