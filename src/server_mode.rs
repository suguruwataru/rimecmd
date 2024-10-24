use crate::json_request_processor::{JsonRequestProcessor, Outcome, Reply};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_data::ReadData;
use crate::rime_api::RimeSession;
use crate::Effect;
use crate::{Error, Result};
use std::io::Write;
use std::os::unix::net::UnixStream;

pub struct ServerMode<'a> {
    pub json_source: JsonSource<UnixStream>,
    pub json_dest: UnixStream,
    pub rime_session: RimeSession<'a>,
}

impl<'a> ServerMode<'a> {
    pub fn main(mut self) -> Result<()> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        loop {
            let request = self.json_source.read_data();
            let reply = match request {
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
            if let Reply {
                outcome: Outcome::Effect(Effect::Stop),
                ..
            } = reply
            {
                break;
            }
        }
        match self.json_source.read_data() {
            Err(Error::InputClosed) => Ok(()),
            Ok(_) => Err(Error::ClientShouldCloseConnection),
            other => other,
        }
    }
}
