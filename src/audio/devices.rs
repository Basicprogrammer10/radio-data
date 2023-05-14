//! Contains the [`Devices`] struct and the [`get_devices`] function.
//! Used at startup to pick the input and output devices.

use clap::ArgMatches;
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, SupportedStreamConfig,
};

use crate::misc::Similarity;

/// Information about the input and output devices being used.
/// Holds the [`Device`] and [`SupportedStreamConfig`] struct from cpal and a gain value which is applied in the Module's [`crate::modules::Module::input_raw`] and [`crate::modules::Module::output_raw`] functions.
pub struct Devices {
    /// cpal device for input
    pub input_device: Device,
    /// clap stream config for input
    pub input_config: SupportedStreamConfig,
    /// Input gain that should be applied to the input data before processing
    pub input_gain: f32,

    /// cpal device for output
    pub output_device: Device,

    /// clap stream config for output
    pub output_config: SupportedStreamConfig,
    /// Output gain that should be applied to the output data after processing
    pub output_gain: f32,
}

/// Uses the command line flags (-i and -o) to pick the audio devices, returning a [`Devices`] struct.
/// Note: The devices are picked by finding the device with the highest string similarity (dice coefficient) to the given name.
pub fn get_devices(args: &ArgMatches) -> Devices {
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
        input_gain: *args.get_one::<f32>("input-gain").unwrap(),
        output_config: output_device
            .default_output_config()
            .expect("No default output config"),
        output_device,
        output_gain: *args.get_one::<f32>("output-gain").unwrap(),
    }
}
