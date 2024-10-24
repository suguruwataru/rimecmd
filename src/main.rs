mod error;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
use error::Error;

#[cfg(test)]
mod testing_utilities;

use clap::Parser;
use std::io::{stdout, Write};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(long, short = 'l', value_enum, default_value = "none")]
    /// The lowest level of Rime logs to write to stderr.
    ///
    /// When `none`, no logs will be written.
    rime_log_level: rime_api::LogLevel,
}

fn main() {
    let args = Args::parse();
    let data_home = xdg::BaseDirectories::with_prefix("rimecmd")
        .map_err(|err| Error::External(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = rime_api::RimeApi::new(&data_home, "/usr/share/rime-data", args.rime_log_level);
    let mut terminal_interface = terminal_interface::TerminalInterface::new(
        key_processor::KeyProcessor::new(rime_api::RimeSession::new(&rime_api)),
    )
    .unwrap();
    if let Some(commit_string) = terminal_interface.process_input().unwrap() {
        writeln!(stdout(), "{}", commit_string).unwrap();
    }
}
