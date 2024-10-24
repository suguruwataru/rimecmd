use crate::json_mode::{Bytes, ServerReader, Stdin};
use crate::json_request_processor::{Outcome, Reply, Request};
use crate::poll_data::{PollData, ReadData};
use crate::terminal_interface::TerminalInterface;
use crate::Result;
use crate::{Config, Effect};
use std::cell::RefCell;
use std::io::{stdin, stdout, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::rc::Rc;

pub enum Data {
    TerminalRequest(Request),
    StdinBytes(Vec<u8>),
    ServerBytes(Vec<u8>),
}

impl ReadData<Data> for Stdin {
    fn read_data(&mut self) -> Result<Data> {
        let Bytes::StdinBytes(bytes) = ReadData::<Bytes>::read_data(self)? else {
            unreachable!()
        };
        Ok(Data::StdinBytes(bytes))
    }
    fn register(&self, poll_data: &mut PollData<Data>) -> Result<()> {
        poll_data.register(&self.stdin)?;
        Ok(())
    }
}

impl ReadData<Data> for ServerReader {
    fn read_data(&mut self) -> Result<Data> {
        let Bytes::ServerBytes(bytes) = ReadData::<Bytes>::read_data(self)? else {
            unreachable!()
        };
        Ok(Data::ServerBytes(bytes))
    }
    fn register(&self, poll_data: &mut PollData<Data>) -> Result<()> {
        poll_data.register(&self.stream)?;
        Ok(())
    }
}

pub struct TerminalJsonMode {
    config: Config,
    terminal_interface: Rc<RefCell<TerminalInterface>>,
}

impl TerminalJsonMode {
    pub fn new(config: Config, terminal_interface: TerminalInterface) -> Self {
        Self {
            config,
            terminal_interface: Rc::new(RefCell::new(terminal_interface)),
        }
    }

    pub fn main(mut self) -> Result<()> {
        match self.main_impl() {
            Ok(()) => Ok(()),
            Err(err) => {
                self.terminal_interface.borrow_mut().close()?;
                Err(err)
            }
        }
    }

    fn main_impl(&mut self) -> Result<()> {
        self.terminal_interface.borrow_mut().open()?;
        let stream = UnixStream::connect(&self.config.unix_socket)?;
        let mut server_writer = stream.try_clone()?;
        let server_reader = Rc::new(RefCell::new(ServerReader { stream }));
        let stdin = Rc::new(RefCell::new(Stdin { stdin: stdin() }));
        let mut poll_data = PollData::new(&[
            Rc::clone(&self.terminal_interface) as Rc<RefCell<dyn ReadData<Data>>>,
            Rc::clone(&stdin) as Rc<RefCell<dyn ReadData<Data>>>,
            Rc::clone(&server_reader) as Rc<RefCell<dyn ReadData<Data>>>,
        ])?;
        loop {
            let data = poll_data.poll()?;
            let reply = match data {
                Data::TerminalRequest(request) => {
                    server_writer.write(serde_json::to_string(&request).unwrap().as_bytes())?;
                    server_writer.flush()?;
                    continue;
                }
                Data::StdinBytes(bytes) => {
                    server_writer.write(&bytes)?;
                    server_writer.flush()?;
                    continue;
                }
                Data::ServerBytes(bytes) => {
                    stdout().write(&bytes)?;
                    stdout().flush()?;
                    serde_json::from_slice(&bytes)?
                }
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::CommitString(_)),
                    ..
                } => {
                    if !self.config.continue_mode {
                        self.terminal_interface.borrow_mut().close()?;
                        stdout().write(&serde_json::to_string(&reply)?.as_bytes())?;
                        stdout().flush()?;
                        server_reader.borrow_mut().stream.shutdown(Shutdown::Both)?;
                        break;
                    } else {
                        self.terminal_interface.borrow_mut().remove_ui()?;
                        stdout().write(&serde_json::to_string(&reply)?.as_bytes())?;
                        stdout().flush()?;
                        self.terminal_interface.borrow_mut().setup_ui()?;
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
                    stdout().write(&serde_json::to_string(&reply)?.as_bytes())?;
                    stdout().flush()?;
                    self.terminal_interface
                        .borrow_mut()
                        .update_ui(composition, menu)?;
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient | Effect::StopServer),
                    ..
                } => {
                    stdout().write(&serde_json::to_string(&reply)?.as_bytes())?;
                    stdout().flush()?;
                    server_reader.borrow_mut().stream.shutdown(Shutdown::Both)?;
                    self.terminal_interface.borrow_mut().close()?;
                    break;
                }
                reply => {
                    stdout().write(&serde_json::to_string(&reply)?.as_bytes())?;
                    stdout().flush()?;
                }
            }
        }
        Ok(())
    }
}
