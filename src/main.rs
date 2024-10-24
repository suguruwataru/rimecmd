#[allow(dead_code)]
mod rime_api;

mod error;
use error::Error;

mod request_handler;
mod terminal_interface;

#[cfg(test)]
mod testing_utilities;

use std::io::{Read, Write};

struct View {
    input_characters: Vec<char>,
    height: usize,
}

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
    std::io::stdin().bytes().take(2).fold(
        View {
            input_characters: vec![],
            height: 0,
        },
        |view, maybe_byte| {
            let character = maybe_byte.unwrap() as char;
            let response = terminal_interface.handle_character(character);
            terminal_interface.erase_line().unwrap();
            terminal_interface.carriage_return().unwrap();
            terminal_interface.cursor_up(view.height).unwrap();
            let view = match response {
                request_handler::Response::ProcessKey {
                    commit_text: _,
                    preview_text: _,
                    menu,
                } => {
                    menu.candidates
                        .iter()
                        .take(menu.page_size)
                        .enumerate()
                        .for_each(|(index, candidate)| {
                            write!(terminal_interface, "{}. {}\r\n", index + 1, candidate.text,)
                                .unwrap();
                        });
                    View {
                        input_characters: view
                            .input_characters
                            .into_iter()
                            .chain(std::iter::once(character))
                            .collect(),
                        height: menu.page_size,
                    }
                }
                _ => unimplemented!(),
            };
            write!(
                terminal_interface,
                "> {}",
                view.input_characters.iter().collect::<String>()
            )
            .unwrap();
            view
        },
    );
    terminal_interface.carriage_return().unwrap();
    write!(terminal_interface, "\n").unwrap();
    terminal_interface.flush().unwrap();
    terminal_interface.exit_raw_mode().unwrap();
}
