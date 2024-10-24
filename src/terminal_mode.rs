use crate::client::{Client, ReplyState};
use crate::json_request_processor::{Outcome, Reply, Request};
use crate::poll_data::ReadData;
use crate::terminal_interface::TerminalInterface;
use crate::{Call, Effect, Error};
use std::io::{stdout, Write};
use uuid::Uuid;

pub struct TerminalMode {
    client: Client,
    terminal_interface: TerminalInterface,
}

impl TerminalMode {
    pub fn new(client: Client, terminal_interface: TerminalInterface) -> Self {
        Self {
            client,
            terminal_interface,
        }
    }

    pub fn main(mut self, continue_mode: bool) -> Result<(), Error> {
        match Self::main_impl(self.client, &mut self.terminal_interface, continue_mode) {
            Ok(()) => Ok(()),
            Err(err) => {
                self.terminal_interface.close()?;
                Err(err)
            }
        }
    }

    fn main_impl(
        mut client: Client,
        terminal_interface: &mut TerminalInterface,
        continue_mode: bool,
    ) -> Result<(), Error> {
        terminal_interface.open()?;
        loop {
            let call = terminal_interface.next_call()?;
            let reply = match call {
                call @ (Call::ProcessKey { .. } | Call::StopClient) => {
                    client.send_bytes(
                        serde_json::to_string(&Request {
                            id: Uuid::new_v4().into(),
                            call,
                        })?
                        .as_bytes(),
                    )?;
                    match client.read_data()? {
                        ReplyState::Complete(reply) => reply,
                        ReplyState::Incomplete => continue,
                    }
                }
                _ => unreachable!(),
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::CommitString(commit_string)),
                    ..
                } => {
                    if !continue_mode {
                        terminal_interface.close()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        break;
                    } else {
                        terminal_interface.remove_ui()?;
                        writeln!(stdout(), "{}", commit_string)?;
                        terminal_interface.setup_ui()?;
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
                    terminal_interface.update_ui(composition, menu)?;
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient),
                    ..
                } => {
                    break;
                }
                _ => (),
            }
        }
        client.shutdown()?;
        terminal_interface.close()?;
        Ok(())
    }
}
