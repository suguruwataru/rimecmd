#[allow(dead_code)]
mod rime_api;

mod error;
use error::Error;

mod request_handler;
mod terminal_interface;

#[cfg(test)]
mod testing_utilities;

use std::io::Write;

struct View {
    input_bytes: Vec<u8>,
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
    let mut view = View {
        input_bytes: vec![],
        height: 0,
    };
    loop {
        let Some((response, byte_vec)) = terminal_interface.next_response() else {
            unimplemented!();
        };
        terminal_interface.erase_line().unwrap();
        terminal_interface.carriage_return().unwrap();
        terminal_interface.cursor_up(view.height).unwrap();
        match response {
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
                view = View {
                    input_bytes: view
                        .input_bytes
                        .into_iter()
                        .chain(byte_vec.into_iter())
                        .collect(),
                    height: menu.page_size,
                }
            }
            request_handler::Response::Exit => {
                terminal_interface.carriage_return().unwrap();
                terminal_interface.erase_after().unwrap();
                terminal_interface.flush().unwrap();
                terminal_interface.exit_raw_mode().unwrap();
                return;
            }
            _ => unimplemented!(),
        };
        terminal_interface
            .write(&[b"> ", view.input_bytes.as_slice()].concat())
            .unwrap();
    }
    terminal_interface.carriage_return().unwrap();
    write!(terminal_interface, "\n").unwrap();
    terminal_interface.flush().unwrap();
    terminal_interface.exit_raw_mode().unwrap();
}
