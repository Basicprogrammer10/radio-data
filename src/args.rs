//! Command line argument parsing

use std::{num::ParseIntError, ops::Range, process, sync::Arc};

use anyhow::Context;
use clap::{value_parser, Arg, ArgMatches, Command};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    SupportedStreamConfig,
};

use crate::{
    audio::windows::{self, Window},
    modules::{
        dtmf::{dtmf_receive, dtmf_send},
        morse::{morse_receive, morse_send},
        range_test, spectrum_analyzer, true_random, InitContext, Module,
    },
};

/// Object-safe module type
type BoxedModule = Box<Arc<dyn Module + Send + Sync + 'static>>;

/// Parse command line args
pub fn parse_args() -> ArgMatches {
    Command::new("radio-data")
        .author("Connor Slade")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .arg(
            Arg::new("input-device")
                .short('i')
                .help("The input device to use.")
                .default_value("default"),
        )
        .arg(
            Arg::new("output-device")
                .short('o')
                .help("The output device to use.")
                .default_value("default"),
        )
        .arg(
            Arg::new("input-gain")
                .long("ig")
                .help("The gain to apply to the input device.")
                .default_value("1.0")
                .value_parser(value_parser!(f32)),
        )
        .arg(
            Arg::new("output-gain")
                .long("og")
                .help("The gain to apply to the output device.")
                .default_value("1.0")
                .value_parser(value_parser!(f32)),
        )
        .subcommands([
            Command::new("device")
                .alias("dev")
                .about("Lists the available audio devices."),
            Command::new("range")
                .alias("r")
                .about("Lets you test the range of your radio system."),
            Command::new("dtmf")
                .alias("d")
                .subcommand_required(true)
                .subcommands([
                    Command::new("send")
                        .alias("s")
                        .about("Sends DTMF tones to the radio.")
                        .arg(
                            Arg::new("data")
                                .help("The data to send.")
                                .required(true)
                                .index(1),
                        ),
                    Command::new("receive")
                        .alias("r")
                        .about("Receives DTMF tones from the radio."),
                ]),
            Command::new("spectrum")
                .alias("s")
                .about("Shows a spectrum analyzer in the terminal")
                .arg(
                    Arg::new("fft-size")
                        .short('f')
                        .help("The sample size of the FFT. Should be a power of 2.")
                        .value_parser(value_parser!(usize))
                        .default_value("2048"),
                )
                .arg(
                    Arg::new("display-range")
                        .short('d')
                        .help("The range of frequencies to display. In the format of `low..high`.")
                        .value_parser(|x: &str| {
                            let mut x = x.split("..");
                            let start = x.next().unwrap().parse::<usize>()?;
                            let end = x.next().unwrap().parse::<usize>()?;
                            Ok::<Range<usize>, ParseIntError>(start..end)
                        })
                        .default_value("15..14000"),
                )
                .arg(
                    Arg::new("window")
                        .short('w')
                        .help("The window function to use on the samples")
                        .value_parser(|x: &str| {
                            let window = windows::get_window(x).with_context(|| {
                                format!("Must be: {}", windows::WINDOWS.join(", "))
                            })?;
                            Ok::<Arc<Box<dyn Window + Send + Sync + 'static>>, anyhow::Error>(
                                Arc::new(window),
                            )
                        })
                        .default_value("hann"),
                )
                .arg(
                    Arg::new("passthrough")
                        .short('p')
                        .help("Pass the audio through to the output device.")
                        .num_args(0),
                )
                .arg(
                    Arg::new("gain")
                        .short('g')
                        .help("The gain to apply display, does not affect the passthrough.")
                        .value_parser(value_parser!(f32))
                        .default_value("1.0"),
                )
                .arg(
                    Arg::new("display-type")
                        .short('t')
                        .help("The method to use to display the spectrum.")
                        .value_parser(value_parser!(spectrum_analyzer::DisplayType)),
                ),
            Command::new("true-random")
                .alias("trng")
                .alias("t")
                .about("Generates a true random numbers.")
                .disable_help_flag(true)
                .arg(
                    Arg::new("host")
                        .short('h')
                        .help("The host to serve on.")
                        .default_value("localhost"),
                )
                .arg(
                    Arg::new("port")
                        .short('p')
                        .help("The port to serve on.")
                        .value_parser(value_parser!(u16))
                        .default_value("8080"),
                )
                .arg(
                    Arg::new("threads")
                        .short('t')
                        .help("The number of threads to use.")
                        .value_parser(value_parser!(usize))
                        .default_value("1"),
                )
                .arg(
                    Arg::new("buffer-size")
                        .short('b')
                        .help("The size of the buffer to use.")
                        .value_parser(value_parser!(usize))
                        .default_value("1024"),
                ),
            Command::new("morse-code")
                .alias("morse")
                .alias("m")
                .about("Transmits text using morse code")
                .subcommand_required(true)
                .arg(
                    Arg::new("dit")
                        .short('d')
                        .help("The length of a dit in milliseconds.")
                        .value_parser(value_parser!(u64))
                        .default_value("100"),
                )
                .arg(
                    Arg::new("frequency")
                        .short('f')
                        .help("The frequency to transmit at.")
                        .value_parser(value_parser!(f32))
                        .default_value("1000"),
                )
                .subcommands([
                    Command::new("send").alias("s").arg(
                        Arg::new("text")
                            .help("The text to transmit")
                            .required(true)
                            .index(1),
                    ),
                    Command::new("receive").alias("r"),
                ]),
        ])
        .get_matches()
}

/// Uses the args to pick the correct module and return it as a boxed trait object
pub fn get_module(
    args: &ArgMatches,
    input: SupportedStreamConfig,
    output: SupportedStreamConfig,
) -> BoxedModule {
    let ic = |x: &ArgMatches| InitContext {
        args: x.to_owned(),
        input,
        output,
    };

    match args.subcommand() {
        Some(("device", _)) => {
            devices();
            process::exit(0);
        }
        Some(("range", m)) => Box::new(range_test::RangeTest::new(ic(m))),
        Some(("dtmf", m)) => match m.subcommand() {
            Some(("send", _)) => Box::new(dtmf_send::DtmfSend::new(ic(m))),
            Some(("receive", _)) => Box::new(dtmf_receive::DtmfReceive::new(ic(m))),
            _ => panic!("Invalid Subcommand"),
        },
        Some(("spectrum", m)) => Box::new(spectrum_analyzer::SpectrumAnalyzer::new(ic(m))),
        Some(("true-random", m)) => Box::new(true_random::TrueRandom::new(ic(m))),
        Some(("morse-code", m)) => match m.subcommand() {
            Some(("send", _)) => Box::new(morse_send::MorseSend::new(ic(m))),
            Some(("receive", _)) => Box::new(morse_receive::MorseReceive::new(ic(m))),
            _ => panic!("Invalid Subcommand"),
        },
        _ => panic!("Invalid Subcommand"),
    }
}

/// Prints out the audio host system and the available devices.
fn devices() {
    let host = cpal::default_host();
    println!("[*] Using Host: {}", host.id().name());

    let devices = host.devices().unwrap().collect::<Vec<_>>();
    if devices.is_empty() {
        println!("[*] No devices found.");
        return;
    }

    println!("[*] Devices ({})", devices.len());
    for (i, device) in devices.iter().enumerate() {
        let input = device.default_input_config().is_ok();
        let output = device.default_output_config().is_ok();
        println!(
            " {}─ {}{} {}",
            if i + 1 == devices.len() { "└" } else { "├" },
            if input { "I" } else { "" },
            if output { "O" } else { "" },
            device.name().unwrap()
        );
    }
}
