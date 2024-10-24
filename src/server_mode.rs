use crate::json_request_processor::{JsonRequestProcessor, Reply, Request};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_request::ReadJson;
use crate::rime_api::RimeSession;
use crate::Call;
use crate::Result;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;

pub struct ServerMode<'a, I: Read + AsRawFd, O: Write> {
    pub json_source: JsonSource<I>,
    pub json_dest: O,
    pub rime_session: RimeSession<'a>,
}

impl<'a, I: Read + AsRawFd, O: Write> ServerMode<'a, I, O> {
    pub fn main(&mut self) -> Result<()> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        loop {
            let request = self.json_source.read_json();
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
            self.json_dest
                .write(serde_json::to_string(&reply)?.as_bytes())?;
            self.json_dest.flush()?;
        }
        Ok(())
    }
}
