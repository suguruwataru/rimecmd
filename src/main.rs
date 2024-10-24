#[allow(dead_code)]
mod rime_api;

mod error;
use error::Error;

mod request_handler;
mod terminal_interface;

#[cfg(test)]
mod testing_utilities;

use std::io::{Read, Write};

fn main() {
    let data_home = xdg::BaseDirectories::with_prefix("rimed")
        .map_err(|err| Error::External(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = rime_api::RimeApi::new(
        &data_home,
        "/usr/share/rime-data",
        rime_api::LogLevel::WARNING,
    );
    let mut terminal_interface = terminal_interface::TerminalInterface::new(
        request_handler::RequestHandler::new(rime_api::RimeSession::new(&rime_api)),
    )
    .unwrap();
    println!("{:?}", data_home);
    terminal_interface.enter_raw_mode().unwrap();
    print!("Input something > ");
    std::io::stdout().flush().unwrap();
    print!(
        "{:?}\n\r",
        terminal_interface
            .handle_character(std::io::stdin().bytes().nth(0).unwrap().unwrap() as char)
    );
    std::io::stdout().flush().unwrap();
    terminal_interface.exit_raw_mode().unwrap();
}
