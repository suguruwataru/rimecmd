use crate::json_request_processor::{Outcome, Reply, Request};
use crate::json_source::JsonSource;
use crate::poll_data::ReadData;
use crate::terminal_interface::TerminalInterface;
use crate::{Call, Config, Effect, Error};
use std::io::{stdout, Write};
use std::os::unix::net::UnixStream;
use uuid::Uuid;

pub struct TerminalMode {
    config: Config,
    terminal_interface: TerminalInterface,
}

impl TerminalMode {
    pub fn new(config: Config, terminal_interface: TerminalInterface) -> Self {
        Self {
            config,
            terminal_interface,
        }
    }

    pub fn main(mut self) -> Result<(), Error> {
        match self.main_impl() {
            Ok(()) => Ok(()),
            Err(err) => {
                self.terminal_interface.close()?;
                Err(err)
            }
        }
    }

    fn main_impl(&mut self) -> Result<(), Error> {
        self.terminal_interface.open()?;
        let stream = UnixStream::connect(&self.config.unix_socket)?;
        let mut json_dest = stream.try_clone()?;
        let mut json_source = JsonSource::new(stream);
        loop {
            let call = self.terminal_interface.next_call()?;
            let reply = match call {
                call @ Call::ProcessKey { .. } => {
                    json_dest.write(
                        serde_json::to_string(&Request {
                            id: Uuid::new_v4().into(),
                            call,
                        })?
                        .as_bytes(),
                    )?;
                    json_dest.flush()?;
                    json_source.read_data()?
                }
                _ => unreachable!(),
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::CommitString(commit_string)),
                    ..
                } => {
                    if !self.config.continue_mode {
                        self.terminal_interface.close()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        break;
                    } else {
                        self.terminal_interface.remove_ui()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        self.terminal_interface.setup_ui()?;
                    }
                }
                Reply {
                    outcome:
                        Outcome::Effect(Effect::UpdateUi {
                            ref menu,
                            ref composition,
                        }),
                    ..
                } => {
                    self.terminal_interface.update_ui(composition, menu)?;
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient),
                    ..
                } => {
                    self.terminal_interface.close()?;
                    break;
                }
                _ => (),
            }
        }
        Ok(())
    }
}
