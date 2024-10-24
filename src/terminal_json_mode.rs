use crate::json_request_processor::{JsonRequestProcessor, Outcome as ReplyResult, Reply, Request};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_request::{PollRequest, RequestSource};
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::Result;
use crate::{Args, Call, Effect};
use std::cell::RefCell;
use std::io::{stdout, Read, Write};
use std::os::fd::AsRawFd;
use std::rc::Rc;

pub struct TerminalJsonMode<'a, R: Read + AsRawFd> {
    args: Args,
    terminal_interface: Rc<RefCell<TerminalInterface>>,
    json_source: Rc<RefCell<JsonSource<R>>>,
    rime_session: RimeSession<'a>,
}

impl<'a, R: Read + AsRawFd + 'static> TerminalJsonMode<'a, R> {
    pub fn new(
        args: Args,
        terminal_interface: TerminalInterface,
        json_source: JsonSource<R>,
        rime_session: RimeSession<'a>,
    ) -> Self {
        Self {
            args,
            terminal_interface: Rc::new(RefCell::new(terminal_interface)),
            json_source: Rc::new(RefCell::new(json_source)),
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
            Rc::clone(&self.json_source) as Rc<RefCell<dyn RequestSource>>,
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
