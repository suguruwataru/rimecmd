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
    std::io::stdin()
        .bytes()
        .take(2)
        .map(|maybe_byte| {
            let character = maybe_byte.unwrap() as char;
            let response = terminal_interface.handle_character(character);
            std::io::stdout().write(b"\r\x1b[0K").unwrap();
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
                        write!(std::io::stdout(), "{}. {}\r\n", index + 1, candidate.text,)
                            .unwrap();
                    }),
                _ => unimplemented!(),
            }
            character
        })
        .fold(vec![], |sequence, character| {
            let sequence: Vec<_> = sequence
                .into_iter()
                .chain(std::iter::once(character))
                .collect();
            write!(
                std::io::stdout(),
                "> {}",
                sequence.iter().collect::<String>()
            )
            .unwrap();
            std::io::stdout().flush().unwrap();
            sequence
        });
    write!(std::io::stdout(), "\r\n").unwrap();
    std::io::stdout().flush().unwrap();
    terminal_interface.exit_raw_mode().unwrap();
}
