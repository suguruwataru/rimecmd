use crate::json_request_processor::{JsonRequestProcessor, Outcome, Reply};
use crate::json_source::JsonSource;
use crate::key_processor::KeyProcessor;
use crate::poll_data::ReadData;
use crate::rime_api::{RimeApi, RimeSession};
use crate::Config;
use crate::Effect;
use crate::{Error, Result};
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;

pub struct ServerMode {
    config: Config,
}

impl ServerMode {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn main(self) -> Result<()> {
        let listener = UnixListener::bind(&self.config.unix_socket).unwrap();
        let (error_sender, error_receiver) = channel();
        let error_sender = Arc::new(Mutex::new(error_sender));
        thread::spawn(move || loop {
            let _error: Error = error_receiver.recv().unwrap();
            todo!("implement error logging");
        });
        let rime_api = Arc::new(Mutex::new(RimeApi::new(
            &self.config.user_data_directory,
            "/usr/share/rime-data",
            self.config.rime_log_level,
        )));
        for stream in listener.incoming() {
            let stream = stream?;
            let error_sender = Arc::clone(&error_sender);
            let rime_api = Arc::clone(&rime_api);
            thread::spawn(move || {
                let rime_session = RimeSession::new(rime_api);
                let input_stream = match stream.try_clone() {
                    Ok(stream) => stream,
                    Err(error) => {
                        error_sender.lock().unwrap().send(error.into()).unwrap();
                        return ();
                    }
                };
                Session {
                    json_source: JsonSource::new(input_stream),
                    json_dest: stream,
                    rime_session,
                }
                .run()
                .unwrap_or_else(|err| error_sender.lock().unwrap().send(err).unwrap());
            });
        }
        Ok(())
    }
}

struct Session {
    json_source: JsonSource<UnixStream>,
    json_dest: UnixStream,
    rime_session: RimeSession,
}

impl Session {
    pub fn run(mut self) -> Result<()> {
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
