use crate::client::Client;
use crate::json_request_processor::{Outcome, Reply};
use crate::Effect;
use crate::Result;
use std::cell::RefCell;
use std::io::{stdin, stdout, Read, Write};
use std::os::unix::net::UnixStream;
use std::rc::Rc;

use crate::client::ServerReply;
use crate::poll_data::{PollData, ReadData};

#[allow(dead_code)]
pub enum Bytes {
    StdinBytes(Vec<u8>),
    ServerBytes(Vec<u8>),
}

pub enum Input {
    Stdin(Vec<u8>),
    ServerReply(ServerReply),
}

impl From<ServerReply> for Input {
    fn from(source: ServerReply) -> Self {
        Self::ServerReply(source)
    }
}

pub struct JsonMode {
    client: Client,
    continue_mode: bool,
}

pub struct Stdin {
    pub stdin: std::io::Stdin,
}

pub struct ServerReader {
    pub stream: UnixStream,
}

impl ReadData<Input> for Stdin {
    fn read_data(&mut self) -> Result<Input> {
        let mut buf = [0; 1024];
        let count = self.stdin.read(&mut buf)?;
        Ok(Input::Stdin(Vec::from(&buf[0..count])))
    }
    fn register(&self, poll_data: &mut PollData<Input>) -> Result<()> {
        poll_data.register(&self.stdin)?;
        Ok(())
    }
}

impl ReadData<Bytes> for ServerReader {
    fn read_data(&mut self) -> Result<Bytes> {
        let mut buf = [0; 1024];
        let count = self.stream.read(&mut buf)?;
        Ok(Bytes::ServerBytes(Vec::from(&buf[0..count])))
    }
    fn register(&self, poll_data: &mut PollData<Bytes>) -> Result<()> {
        poll_data.register(&self.stream)?;
        Ok(())
    }
}

impl JsonMode {
    pub fn new(client: Client, continue_mode: bool) -> Self {
        Self {
            client,
            continue_mode,
        }
    }

    pub fn main(self) -> Result<()> {
        let client = Rc::new(RefCell::new(self.client));
        let stdin = Rc::new(RefCell::new(Stdin { stdin: stdin() }));
        let mut poll_data = PollData::<Input>::new(&[
            Rc::clone(&client) as Rc<RefCell<dyn ReadData<Input>>>,
            Rc::clone(&stdin) as Rc<RefCell<dyn ReadData<Input>>>,
        ])?;
        loop {
            let data = poll_data.poll()?;
            match data {
                Input::Stdin(bytes) => {
                    client.borrow_mut().send_bytes(&bytes)?;
                }
                Input::ServerReply(ServerReply::Complete(reply)) => {
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
                            if !self.continue_mode {
                                break;
                            }
                        }
                        _ => (),
                    }
                }
                Input::ServerReply(ServerReply::Incomplete) => continue,
            }
        }
        drop(poll_data);
        Rc::into_inner(client).unwrap().into_inner().shutdown()?;
        Ok(())
    }
}
