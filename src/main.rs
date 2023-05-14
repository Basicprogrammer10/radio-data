#![warn(missing_docs)]

use cpal::traits::{DeviceTrait, StreamTrait};

use crate::audio::devices::get_devices;

mod args;
pub mod audio;
mod coding;
mod consts;
mod misc;
mod modules;

fn main() {
    // Get and parse args
    let args = args::parse_args();
    let devices = get_devices(&args);

    // Get module
    let module = args::get_module(
        &args,
        devices.input_config.clone(),
        devices.output_config.clone(),
    );
    println!("[*] Running module `{}`", module.name());

    println!(
        "[*] Input  hooked into `{}` ({}) [{}]",
        devices.input_device.name().unwrap(),
        devices.input_config.sample_rate().0,
        devices.input_config.channels()
    );
    println!(
        "[*] Output hooked into `{}` ({}) [{}]",
        devices.output_device.name().unwrap(),
        devices.output_config.sample_rate().0,
        devices.output_config.channels()
    );

    // Init module and IO streams
    module.init();
    let module_ref = module.clone();
    let output_stream = devices
        .output_device
        .build_output_stream(
            &devices.output_config.into(),
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                module_ref.output_raw(data, info, devices.output_gain)
            },
            move |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    let input_stream = devices
        .input_device
        .build_input_stream(
            &devices.input_config.into(),
            move |data: &[f32], info: &cpal::InputCallbackInfo| {
                module.input_raw(data, info, devices.input_gain)
            },
            |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    output_stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}
