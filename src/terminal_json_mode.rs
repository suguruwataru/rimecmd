use crate::json_request_processor::{JsonRequestProcessor, Outcome, Reply, Request};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_request::{PollRequest, ReadJson};
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::Result;
use crate::{Call, Config, Effect};
use std::cell::RefCell;
use std::io::{Stdin, Write};
use std::rc::Rc;

pub struct TerminalJsonMode<'a, O: Write> {
    config: Config,
    terminal_interface: Rc<RefCell<TerminalInterface>>,
    json_source: Rc<RefCell<JsonSource<Stdin>>>,
    json_dest: O,
    rime_session: RimeSession<'a>,
}

impl<'a, O: Write> TerminalJsonMode<'a, O> {
    pub fn new(
        config: Config,
        terminal_interface: TerminalInterface,
        json_source: JsonSource<Stdin>,
        json_dest: O,
        rime_session: RimeSession<'a>,
    ) -> Self {
        Self {
            config,
            terminal_interface: Rc::new(RefCell::new(terminal_interface)),
            json_source: Rc::new(RefCell::new(json_source)),
            json_dest,
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
            Rc::clone(&self.terminal_interface) as Rc<RefCell<dyn ReadJson<Request>>>,
            Rc::clone(&self.json_source) as Rc<RefCell<dyn ReadJson<Request>>>,
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
                    outcome: Outcome::Effect(Effect::CommitString(_)),
                    ..
                } => {
                    if !self.config.continue_mode {
                        self.terminal_interface.borrow_mut().close()?;
                        writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                        break;
                    } else {
                        self.terminal_interface.borrow_mut().remove_ui()?;
                        writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
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
                    writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                    self.terminal_interface
                        .borrow_mut()
                        .update_ui(composition, menu)?;
                }
                reply => {
                    writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                }
            }
        }
        Ok(())
    }
}
