#[allow(dead_code)]
mod rime_api;

#[allow(dead_code)]
mod key_processor;

mod error;
use error::Error;

mod terminal_interface;

#[cfg(test)]
mod testing_utilities;
#[cfg(test)]
mod tests;

use std::io::Write;

struct View {
    // the string that is input by the user, returned by rime.
    preedit: String,
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
        key_processor::KeyProcessor::new(rime_api::RimeSession::new(&rime_api)),
    )
    .unwrap();
    terminal_interface.open().unwrap();
    let mut view = View {
        preedit: "".into(),
        height: 0,
    };
    loop {
        let Some(action) = terminal_interface.next_response() else {
            unimplemented!();
        };
        terminal_interface.erase_all_line().unwrap();
        terminal_interface.carriage_return().unwrap();
        terminal_interface.cursor_up(view.height).unwrap();
        terminal_interface.erase_after().unwrap();
        match action {
            terminal_interface::Action::Update(key_processor::Report {
                commit_text: _,
                preedit,
                menu,
            }) => {
                menu.candidates
                    .iter()
                    .take(menu.page_size)
                    .enumerate()
                    .for_each(|(index, candidate)| {
                        write!(terminal_interface, "{}. {}\r\n", index + 1, candidate.text,)
                            .unwrap();
                        terminal_interface.erase_line_to_right().unwrap();
                    });
                view = View {
                    preedit,
                    height: menu.page_size,
                }
            }
            terminal_interface::Action::Exit => {
                terminal_interface.carriage_return().unwrap();
                terminal_interface.erase_after().unwrap();
                terminal_interface.flush().unwrap();
                terminal_interface.exit_raw_mode().unwrap();
                return;
            }
        };
        write!(terminal_interface, "> {}", view.preedit).unwrap();
    }
    #[allow(unreachable_code)]
    {
        terminal_interface.carriage_return().unwrap();
        terminal_interface.write(b"\n").unwrap();
        terminal_interface.flush().unwrap();
        terminal_interface.exit_raw_mode().unwrap();
    }
}
