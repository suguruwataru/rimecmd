mod error;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
use error::Error;

#[cfg(test)]
mod testing_utilities;

use std::io::{stdout, Write};

fn main() {
    let data_home = xdg::BaseDirectories::with_prefix("rimecmd")
        .map_err(|err| Error::External(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = rime_api::RimeApi::new(
        &data_home,
        "/usr/share/rime-data",
        rime_api::LogLevel::WARNING,
    );
    let mut terminal_interface = terminal_interface::TerminalInterface::new(
        key_processor::KeyProcessor::new(rime_api::RimeSession::new(&rime_api)),
    )
    .unwrap();
    if let Some(commit_string) = terminal_interface.process_input().unwrap() {
        writeln!(stdout(), "{}", commit_string).unwrap();
    }
}
