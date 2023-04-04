use std::sync::Arc;

use clap::Command;
use cpal::SupportedStreamConfig;

use crate::modules::{range_test, InitContext, Module};

pub fn parse_args(
    input: SupportedStreamConfig,
    output: SupportedStreamConfig,
) -> Box<Arc<dyn Module + Send + Sync + 'static>> {
    let m = Command::new("radio-data")
        .author("Connor Slade")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(true)
        .subcommands([Command::new("range")
            .alias("r")
            .about("Lets you test the range if your radio system.")])
        .get_matches();

    let ic = |x| InitContext {
        args: x,
        input,
        output,
    };

    match m.subcommand() {
        Some(("range", m)) => Box::new(range_test::RangeTest::new(ic(m.to_owned()))),
        _ => panic!("Invalid Subcommand"),
    }
}
