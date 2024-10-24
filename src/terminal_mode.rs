use crate::json_request_processor::{Outcome, Reply, Request};
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::{Call, Config, Effect, Error};
use std::io::{stdout, Read, Write};
use std::os::unix::net::UnixStream;
use uuid::Uuid;

pub struct TerminalMode<'a> {
    pub config: Config,
    pub terminal_interface: TerminalInterface,
    pub rime_session: RimeSession<'a>,
}

impl<'a> TerminalMode<'a> {
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
        let mut stream = UnixStream::connect(&self.config.unix_socket)?;
        loop {
            let call = self.terminal_interface.next_call()?;
            let reply = match call {
                call @ Call::ProcessKey { .. } => {
                    let mut buf = [0; 1024];
                    let mut json_bytes = vec![];
                    stream.write(
                        serde_json::to_string(&Request {
                            id: Uuid::new_v4().into(),
                            call,
                        })
                        .unwrap()
                        .as_bytes(),
                    )?;
                    stream.flush()?;
                    loop {
                        let count = stream.read(&mut buf)?;
                        if count == 0 {
                            return Err(Error::ServerClosedConnection);
                        }
                        json_bytes.extend_from_slice(&buf[0..count]);
                        match serde_json::from_slice::<Reply>(&json_bytes) {
                            Ok(reply) => break reply,
                            Err(error) => {
                                if error.is_eof() {
                                    continue;
                                } else {
                                    return Err(error.into());
                                }
                            }
                        }
                    }
                }
                Call::Stop => {
                    self.terminal_interface.close()?;
                    break;
                }
                _ => unreachable!(),
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::CommitString(commit_string)),
                    ..
                } => {
                    if !self.config.continue_mode {
                        writeln!(stdout(), "{}", commit_string)?;
                        self.terminal_interface.close()?;
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
                _ => (),
            }
        }
        Ok(())
    }
}
