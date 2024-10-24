use crate::json_request_processor::{JsonRequestProcessor, Outcome as ReplyResult, Reply, Request};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_request::RequestSource;
use crate::rime_api::RimeSession;
use crate::Result;
use crate::{Args, Call, Effect};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

pub struct JsonMode<'a, I: Read + AsRawFd, O: Write> {
    pub args: Args,
    pub json_source: JsonSource<I>,
    pub json_dest: O,
    pub rime_session: RimeSession<'a>,
}

impl<'a, I: Read + AsRawFd, O: Write> JsonMode<'a, I, O> {
    pub fn new(
        args: Args,
        json_source: JsonSource<I>,
        json_dest: O,
        rime_session: RimeSession<'a>,
    ) -> Self {
        Self {
            args,
            json_source,
            json_dest,
            rime_session,
        }
    }

    pub fn main(&mut self) -> Result<()> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        loop {
            let request = self.json_source.next_request();
            let reply = match request {
                Ok(Request {
                    id: _,
                    call: Call::Stop,
                }) => {
                    break;
                }
                Ok(request) => json_request_processor.process_request(request),
                Err(err) => match err.try_into() {
                    Ok(err_outcome) => Reply {
                        id: None,
                        outcome: err_outcome,
                    },
                    Err(err) => {
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
                        writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                        break;
                    } else {
                        writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                    }
                }
                reply => {
                    writeln!(self.json_dest, "{}", &serde_json::to_string(&reply)?)?;
                }
            }
        }
        Ok(())
    }
}
