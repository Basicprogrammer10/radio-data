use clap::ArgMatches;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SupportedStreamConfig,
};
use misc::Similarity;

mod args;
mod audio;
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
        devices.output_config.clone(),
        devices.input_config.clone(),
    );
    println!("[*] Running module `{}`", module.name());

    println!(
        "[*] Input  hooked into `{}` ({})",
        devices.input_device.name().unwrap(),
        devices.input_config.sample_rate().0
    );
    println!(
        "[*] Output hooked into `{}` ({})",
        devices.output_device.name().unwrap(),
        devices.output_config.sample_rate().0
    );

    // Init module and IO streams
    module.init();
    let module_ref = module.clone();
    let output_stream = devices
        .output_device
        .build_output_stream(
            &devices.output_config.into(),
            move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| module_ref.output(data),
            move |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    let input_stream = devices
        .input_device
        .build_input_stream(
            &devices.input_config.into(),
            move |data: &[f32], _info: &cpal::InputCallbackInfo| module.input(data),
            |err| eprintln!("[-] Error: {err}"),
            None,
        )
        .unwrap();

    output_stream.play().unwrap();
    input_stream.play().unwrap();
    std::thread::park();
}

struct Devices {
    input_config: SupportedStreamConfig,
    output_config: SupportedStreamConfig,
    input_device: Device,
    output_device: Device,
}

fn get_devices(args: &ArgMatches) -> Devices {
    let host = cpal::default_host();
    let wanted_output_device = args
        .get_one::<String>("output-device")
        .unwrap()
        .to_lowercase();
    let wanted_input_device = args
        .get_one::<String>("input-device")
        .unwrap()
        .to_lowercase();

    let comp_name =
        |dev: &Device, wanted: &String| dev.name().unwrap().to_lowercase().similarity(wanted);

    let output_device = match wanted_output_device.as_str() {
        "default" => host
            .default_output_device()
            .expect("No default output device"),
        _ => {
            host.output_devices()
                .unwrap()
                .map(|x| (comp_name(&x, &wanted_output_device), x))
                .reduce(|a, b| if a.0 > b.0 { a } else { b })
                .expect("No output device found")
                .1
        }
    };

    let input_device = match wanted_input_device.as_str() {
        "default" => host
            .default_input_device()
            .expect("No default input device"),
        _ => {
            host.input_devices()
                .unwrap()
                .map(|x| (comp_name(&x, &wanted_input_device), x))
                .reduce(|a, b| if a.0 > b.0 { a } else { b })
                .expect("No input device found")
                .1
        }
    };

    Devices {
        input_config: input_device
            .default_input_config()
            .expect("No default input config"),
        input_device,
        output_config: output_device
            .default_output_config()
            .expect("No default output config"),
        output_device,
    }
}
