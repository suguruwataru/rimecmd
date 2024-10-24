use crate::client::{Client, ReplyState};
use crate::json_mode::Stdin;
use crate::json_request_processor::{Outcome, Reply, Request};
use crate::poll_data::{PollData, ReadData};
use crate::terminal_interface::TerminalInterface;
use crate::Effect;
use crate::Result;
use std::cell::RefCell;
use std::io::{stdin, stdout, Write};
use std::rc::Rc;

pub enum Input {
    ServerReply(ReplyState),
    StdinBytes(Vec<u8>),
    TerminalRequest(Request),
}

impl From<Vec<u8>> for Input {
    fn from(source: Vec<u8>) -> Self {
        Self::StdinBytes(source)
    }
}

impl From<Request> for Input {
    fn from(source: Request) -> Self {
        Self::TerminalRequest(source)
    }
}

impl From<ReplyState> for Input {
    fn from(source: ReplyState) -> Self {
        Self::ServerReply(source)
    }
}

pub struct TerminalJsonMode {
    terminal_interface: Rc<RefCell<TerminalInterface>>,
    client: Rc<RefCell<Client>>,
}

impl TerminalJsonMode {
    pub fn new(client: Client, terminal_interface: TerminalInterface) -> Self {
        Self {
            client: Rc::new(RefCell::new(client)),
            terminal_interface: Rc::new(RefCell::new(terminal_interface)),
        }
    }

    pub fn main(self, continue_mode: bool) -> Result<()> {
        let Self {
            client,
            terminal_interface,
        } = self;
        match Self::main_impl(client, &terminal_interface, continue_mode) {
            Ok(()) => Ok(()),
            Err(err) => {
                terminal_interface.borrow_mut().close()?;
                Err(err)
            }
        }
    }

    fn main_impl(
        client: Rc<RefCell<Client>>,
        terminal_interface: &Rc<RefCell<TerminalInterface>>,
        continue_mode: bool,
    ) -> Result<()> {
        terminal_interface.borrow_mut().open()?;
        let stdin = Rc::new(RefCell::new(Stdin { stdin: stdin() }));
        let mut poll_data = PollData::new(&[
            Rc::clone(terminal_interface) as Rc<RefCell<dyn ReadData<Input>>>,
            Rc::clone(&stdin) as Rc<RefCell<dyn ReadData<Input>>>,
            Rc::clone(&client) as Rc<RefCell<dyn ReadData<Input>>>,
        ])?;
        loop {
            let data = poll_data.poll()?;
            let reply = match data {
                Input::TerminalRequest(request) => {
                    client
                        .borrow_mut()
                        .send_bytes(serde_json::to_string(&request).unwrap().as_bytes())?;
                    continue;
                }
                Input::StdinBytes(bytes) => {
                    client.borrow_mut().send_bytes(&bytes)?;
                    continue;
                }
                Input::ServerReply(ReplyState::Complete(reply)) => reply,
                Input::ServerReply(ReplyState::Incomplete) => continue,
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::CommitString(_)),
                    ..
                } => {
                    if !continue_mode {
                        stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                        stdout().flush()?;
                        break;
                    } else {
                        terminal_interface.borrow_mut().remove_ui()?;
                        stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                        stdout().flush()?;
                        terminal_interface.borrow_mut().setup_ui()?;
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
                    stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                    stdout().flush()?;
                    terminal_interface
                        .borrow_mut()
                        .update_ui(composition, menu)?;
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient | Effect::StopServer),
                    ..
                } => {
                    stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                    stdout().flush()?;
                    break;
                }
                reply => {
                    stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                    stdout().flush()?;
                }
            }
        }
        drop(poll_data);
        Rc::into_inner(client).unwrap().into_inner().shutdown()?;
        terminal_interface.borrow_mut().close()?;
        Ok(())
    }
}
