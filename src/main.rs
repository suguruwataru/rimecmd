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
    terminal_interface.open().unwrap();
    let response = terminal_interface
        .handle_character(std::io::stdin().bytes().nth(0).unwrap().unwrap() as char);
    write!(std::io::stdout(), "\r\n").unwrap();
    match response {
        request_handler::Response::ProcessKey {
            commit_text: _,
            preview_text: _,
            menu,
        } => menu
            .candidates
            .iter()
            .take(menu.page_size)
            .enumerate()
            .for_each(|(index, candidate)| {
                write!(std::io::stdout(), "{}. {}\r\n", index + 1, candidate.text,).unwrap();
            }),
        _ => unimplemented!(),
    }
    std::io::stdout().flush().unwrap();
    terminal_interface.exit_raw_mode().unwrap();
}
