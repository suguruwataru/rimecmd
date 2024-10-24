use crate::json_request_processor::{JsonRequestProcessor, Outcome as ReplyResult, Reply, Request};
use crate::key_processor::KeyProcessor;
use crate::poll_request::{PollRequest, RequestSource};
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::Result;
use crate::{Args, Call, Effect, Error};
use std::cell::RefCell;
use std::io::{stdout, Read, Write};
use std::os::fd::AsRawFd;
use std::rc::Rc;

pub struct JsonStdin {
    stdin: std::io::Stdin,
}

impl JsonStdin {
    pub fn new() -> Self {
        Self {
            stdin: std::io::stdin(),
        }
    }
}

impl RequestSource for JsonStdin {
    fn register(&self, poll_request: &mut PollRequest) -> Result<()> {
        poll_request.register(&self.stdin.as_raw_fd())
    }

    fn next_request(&mut self) -> Result<Request> {
        let mut buf = [0u8; 1024];
        let mut json_bytes = vec![];
        loop {
            let count = self.stdin.read(&mut buf)?;
            if count == 0 {
                break Err(Error::InputClosed);
            }
            json_bytes.extend_from_slice(&buf[0..count]);
            match serde_json::from_slice::<Request>(&json_bytes) {
                Ok(call) => break Ok(call),
                Err(err) => {
                    if err.is_eof() {
                        continue;
                    }
                    break Err(crate::Error::Json(err));
                }
            };
        }
    }
}

pub struct TerminalJsonMode<'a> {
    args: Args,
    terminal_interface: Rc<RefCell<TerminalInterface>>,
    json_stdin: Rc<RefCell<JsonStdin>>,
    rime_session: RimeSession<'a>,
}

impl<'a> TerminalJsonMode<'a> {
    pub fn new(
        args: Args,
        terminal_interface: TerminalInterface,
        json_stdin: JsonStdin,
        rime_session: RimeSession<'a>,
    ) -> Self {
        Self {
            args,
            terminal_interface: Rc::new(RefCell::new(terminal_interface)),
            json_stdin: Rc::new(RefCell::new(json_stdin)),
            rime_session,
        }
    }

    pub fn main(&mut self) -> Result<()> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        self.terminal_interface.borrow_mut().open()?;
        let mut poll_request = PollRequest::new(&[
            Rc::clone(&self.terminal_interface) as Rc<RefCell<dyn RequestSource>>,
            Rc::clone(&self.json_stdin) as Rc<RefCell<dyn RequestSource>>,
        ])?;
        loop {
            let request = poll_request.poll();
            let reply = match request {
                Ok(Request {
                    id: _,
                    call: Call::Stop,
                }) => {
                    self.terminal_interface.borrow_mut().close()?;
                    break;
                }
                Ok(request) => json_request_processor.process_request(request),
                Err(err) => match err.try_into() {
                    Ok(err_outcome) => Reply {
                        id: None,
                        outcome: err_outcome,
                    },
                    Err(err) => {
                        self.terminal_interface.borrow_mut().close()?;
                        return Err(err);
                    }
                },
            };
            match reply {
                Reply {
                    outcome: ReplyResult::Effect(Effect::CommitString(_)),
                    ..
                } => {
                    if !self.args.continue_mode {
                        self.terminal_interface.borrow_mut().close()?;
                        writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                        break;
                    } else {
                        self.terminal_interface.borrow_mut().remove_ui()?;
                        writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                        self.terminal_interface.borrow_mut().setup_ui()?;
                    }
                }
                Reply {
                    outcome:
                        ReplyResult::Effect(Effect::UpdateUi {
                            ref menu,
                            ref composition,
                        }),
                    ..
                } => {
                    writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                    self.terminal_interface
                        .borrow_mut()
                        .update_ui(composition, menu)?;
                }
                reply => {
                    writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                }
            }
        }
        Ok(())
    }
}
