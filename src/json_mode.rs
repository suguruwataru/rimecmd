use crate::json_request_processor::{Outcome, Reply};
use crate::Config;
use crate::Effect;
use crate::Result;
use std::cell::RefCell;
use std::io::{stdin, stdout, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::rc::Rc;

use crate::poll_data::{PollData, ReadData};

pub enum Bytes {
    StdinBytes(Vec<u8>),
    ServerBytes(Vec<u8>),
}

pub struct JsonMode {
    config: Config,
}

pub struct Stdin {
    pub stdin: std::io::Stdin,
}

pub struct ServerReader {
    pub stream: UnixStream,
}

impl ReadData<Bytes> for Stdin {
    fn read_data(&mut self) -> Result<Bytes> {
        let mut buf = [0; 1024];
        let count = self.stdin.read(&mut buf)?;
        Ok(Bytes::StdinBytes(Vec::from(&buf[0..count])))
    }
    fn register(&self, poll_data: &mut PollData<Bytes>) -> Result<()> {
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
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn main(self) -> Result<()> {
        let stream = UnixStream::connect(&self.config.unix_socket)?;
        let mut server_writer = stream.try_clone()?;
        let server_reader = Rc::new(RefCell::new(ServerReader { stream }));
        let stdin = Rc::new(RefCell::new(Stdin { stdin: stdin() }));
        let mut poll_data = PollData::<Bytes>::new(&[
            Rc::clone(&server_reader) as Rc<RefCell<dyn ReadData<Bytes>>>,
            Rc::clone(&stdin) as Rc<RefCell<dyn ReadData<Bytes>>>,
        ])?;
        loop {
            let bytes = poll_data.poll()?;
            match bytes {
                Bytes::StdinBytes(bytes) => {
                    server_writer.write(&bytes)?;
                    server_writer.flush()?;
                }
                Bytes::ServerBytes(bytes) => {
                    stdout().write(&bytes)?;
                    stdout().flush()?;
                    match serde_json::from_slice(&bytes)? {
                        Reply {
                            outcome: Outcome::Effect(Effect::StopClient | Effect::StopServer),
                            ..
                        } => {
                            server_reader.borrow_mut().stream.shutdown(Shutdown::Both)?;
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }
        Ok(())
    }
}
