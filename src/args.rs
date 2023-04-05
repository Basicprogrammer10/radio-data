use std::sync::Arc;

use clap::{Arg, Command};
use cpal::SupportedStreamConfig;

use crate::modules::{dtmf_receive, dtmf_send, range_test, InitContext, Module};

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
                .about("Lets you test the range if your radio system."),
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
        ])
        .get_matches();

    let ic = |x| InitContext {
        args: x,
        input,
        output,
    };

    match m.subcommand() {
        Some(("range", m)) => Box::new(range_test::RangeTest::new(ic(m.to_owned()))),
        Some(("dtmf-send", m)) => Box::new(dtmf_send::DtmfSend::new(ic(m.to_owned()))),
        Some(("dtmf-receive", m)) => Box::new(dtmf_receive::DtmfReceive::new(ic(m.to_owned()))),
        _ => panic!("Invalid Subcommand"),
    }
}
