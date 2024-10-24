use crate::client::Client;
use crate::json_request_processor::{Outcome, Reply};
use crate::Effect;
use crate::Result;
use std::cell::RefCell;
use std::io::{stdin, stdout, Read, Write};
use std::rc::Rc;

use crate::client::ReplyState;
use crate::poll_data::{PollData, ReadData};

enum Input {
    StdinBytes(Vec<u8>),
    ServerReply(ReplyState),
}

impl From<ReplyState> for Input {
    fn from(source: ReplyState) -> Self {
        Self::ServerReply(source)
    }
}

impl From<Vec<u8>> for Input {
    fn from(source: Vec<u8>) -> Self {
        Self::StdinBytes(source)
    }
}

pub struct JsonMode {
    client: Client,
}

pub struct Stdin {
    pub stdin: std::io::Stdin,
}

impl<D: From<Vec<u8>>> ReadData<D> for Stdin {
    fn read_data(&mut self) -> Result<D> {
        let mut buf = [0; 1024];
        let count = self.stdin.read(&mut buf)?;
        Ok(Vec::from(&buf[0..count]).into())
    }
    fn register(&self, poll_data: &mut PollData<D>) -> Result<()> {
        poll_data.register(&self.stdin)?;
        Ok(())
    }
}

impl JsonMode {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn main(self, continue_mode: bool) -> Result<()> {
        let client = Rc::new(RefCell::new(self.client));
        let stdin = Rc::new(RefCell::new(Stdin { stdin: stdin() }));
        let mut poll_data = PollData::<Input>::new(&[
            Rc::clone(&client) as Rc<RefCell<dyn ReadData<Input>>>,
            Rc::clone(&stdin) as Rc<RefCell<dyn ReadData<Input>>>,
        ])?;
        loop {
            let data = poll_data.poll()?;
            match data {
                Input::StdinBytes(bytes) => {
                    client.borrow_mut().send_bytes(&bytes)?;
                }
                Input::ServerReply(ReplyState::Complete(reply)) => {
                    stdout().write(&serde_json::to_string(&reply).unwrap().as_bytes())?;
                    stdout().flush()?;
                    match reply {
                        Reply {
                            outcome: Outcome::Effect(Effect::StopClient | Effect::StopServer),
                            ..
                        } => {
                            break;
                        }
                        Reply {
                            outcome: Outcome::Effect(Effect::CommitString(_)),
                            ..
                        } => {
                            if !continue_mode {
                                break;
                            }
                        }
                        _ => (),
                    }
                }
                Input::ServerReply(ReplyState::Incomplete) => continue,
            }
        }
        drop(poll_data);
        Rc::into_inner(client).unwrap().into_inner().shutdown()?;
        Ok(())
    }
}
