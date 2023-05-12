use std::{num::ParseIntError, ops::Range, sync::Arc};

use clap::{value_parser, Arg, ArgMatches, Command};
use cpal::SupportedStreamConfig;

use crate::modules::{
    dtmf_receive, dtmf_send, range_test, spectrum_analyzer, true_random, InitContext, Module,
};

pub fn parse_args(
    input: SupportedStreamConfig,
    output: SupportedStreamConfig,
) -> Box<Arc<dyn Module + Send + Sync + 'static>> {
    let m = Command::new("radio-data")
        .author("Connor Slade")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .subcommands([
            Command::new("range")
                .alias("r")
                .about("Lets you test the range of your radio system."),
            Command::new("dtmf-send")
                .alias("ds")
                .about("Sends DTMF tones to the radio.")
                .arg(
                    Arg::new("data")
                        .help("The data to send.")
                        .required(true)
                        .index(1),
                ),
            Command::new("dtmf-receive")
                .alias("dr")
                .about("Receives DTMF tones from the radio."),
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
        ])
        .get_matches();

    let ic = |x: &ArgMatches| InitContext {
        args: x.to_owned(),
        input,
        output,
    };

    match m.subcommand() {
        Some(("range", m)) => Box::new(range_test::RangeTest::new(ic(m))),
        Some(("dtmf-send", m)) => Box::new(dtmf_send::DtmfSend::new(ic(m))),
        Some(("dtmf-receive", m)) => Box::new(dtmf_receive::DtmfReceive::new(ic(m))),
        Some(("spectrum", m)) => Box::new(spectrum_analyzer::SpectrumAnalyzer::new(ic(m))),
        Some(("true-random", m)) => Box::new(true_random::TrueRandom::new(ic(m))),
        _ => panic!("Invalid Subcommand"),
    }
}
