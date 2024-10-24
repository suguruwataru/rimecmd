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
    unix_listener: UnixListener,
}

impl ServerMode {
    pub fn new(config: Config, unix_listener: UnixListener) -> Self {
        Self {
            config,
            unix_listener,
        }
    }

    pub fn main(self) -> Result<()> {
        let (error_sender, error_receiver) = channel();
        let (stop_sender, stop_receiver) = channel();
        let error_sender = Arc::new(Mutex::new(error_sender));
        let stop_sender = Arc::new(Mutex::new(stop_sender));
        thread::spawn(move || loop {
            let _error: Error = error_receiver.recv().unwrap();
            todo!("implement error logging");
        });
        let rime_api = Arc::new(Mutex::new(RimeApi::new(
            &self.config.user_data_directory,
            "/usr/share/rime-data",
            self.config.rime_log_level,
        )));
        thread::spawn(move || {
            for stream in self.unix_listener.incoming() {
                let stream = match stream {
                    Ok(stream) => stream,
                    Err(err) => {
                        error_sender.lock().unwrap().send(err.into()).unwrap();
                        break;
                    }
                };
                let error_sender = Arc::clone(&error_sender);
                let stop_sender = Arc::clone(&stop_sender);
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
                    if (Session {
                        json_source: JsonSource::new(input_stream),
                        json_dest: stream,
                        rime_session,
                    })
                    .run()
                    .unwrap_or_else(|err| {
                        error_sender.lock().unwrap().send(err).unwrap();
                        false
                    }) {
                        stop_sender.lock().unwrap().send(()).unwrap();
                    }
                });
            }
        });
        stop_receiver.recv().unwrap();
        Ok(())
    }
}

struct Session {
    json_source: JsonSource<UnixStream>,
    json_dest: UnixStream,
    rime_session: RimeSession,
}

impl Session {
    /// On success, return whether the whole server should stop.
    pub fn run(mut self) -> Result<bool> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        let stop_server = loop {
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
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient),
                    ..
                } => {
                    break false;
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopServer),
                    ..
                } => {
                    break true;
                }
                _ => (),
            }
        };
        match self.json_source.read_data() {
            Err(Error::InputClosed) => Ok(stop_server),
            Ok(_) => Err(Error::ClientShouldCloseConnection),
            other => other,
        }
    }
}
