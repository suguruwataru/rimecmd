use crate::json_request_processor::{JsonRequestProcessor, Outcome, Reply, Request};
use crate::key_processor::KeyProcessor;
use crate::rime_api::{RimeApi, RimeSession};
use crate::Config;
use crate::Effect;
use crate::{Error, Result};
use signal_hook::consts::signal::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs::remove_file;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::sync::{
    mpsc::{channel, Sender},
    Arc, Mutex,
};
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
        thread::spawn(move || loop {
            let _error: Error = error_receiver.recv().unwrap();
            eprintln!("{:?}", _error);
            todo!("implement error logging");
        });
        let (stop_sender, stop_receiver) = channel();
        let stop_sender = Arc::new(Mutex::new(stop_sender));
        let error_sender = Arc::new(Mutex::new(error_sender));
        let sigterm_stop_sender = Arc::clone(&stop_sender);
        let sigterm_error_sender = Arc::clone(&error_sender);
        thread::spawn(move || {
            let mut signals = match Signals::new(&[SIGTERM, SIGINT]) {
                Ok(signals) => signals,
                Err(err) => {
                    sigterm_error_sender
                        .lock()
                        .unwrap()
                        .send(err.into())
                        .unwrap();
                    return;
                }
            };
            for signal in signals.forever() {
                match signal {
                    SIGTERM | SIGINT => {
                        sigterm_stop_sender.lock().unwrap().send(()).unwrap();
                        return;
                    }
                    _ => unreachable!(),
                }
            }
        });
        let rime_api = Arc::new(Mutex::new(RimeApi::new(
            &self.config.user_data_directory,
            "/usr/share/rime-data",
            self.config.rime_log_level,
        )));
        thread::spawn(move || {
            let client_count = Arc::new(Mutex::new(0));
            for stream in self.unix_listener.incoming() {
                *client_count.lock().unwrap() += 1;
                let stream = match stream {
                    Ok(stream) => stream,
                    Err(err) => {
                        error_sender.lock().unwrap().send(err.into()).unwrap();
                        break;
                    }
                };
                let error_sender = Arc::clone(&error_sender);
                let stop_sender = Arc::clone(&stop_sender);
                let client_count = Arc::clone(&client_count);
                let rime_api = Arc::clone(&rime_api);
                thread::spawn(move || {
                    Session {
                        client_stream: stream,
                        client_count: Arc::clone(&client_count),
                        rime_session: RimeSession::new(rime_api),
                        stop_sender,
                    }
                    .run()
                    .unwrap_or_else(|err| error_sender.lock().unwrap().send(err).unwrap());
                    *client_count.lock().unwrap() -= 1;
                });
            }
        });
        stop_receiver.recv().unwrap();
        remove_file(&self.config.unix_socket)?;
        Ok(())
    }
}

struct Session {
    client_stream: UnixStream,
    client_count: Arc<Mutex<usize>>,
    rime_session: RimeSession,
    stop_sender: Arc<Mutex<Sender<()>>>,
}

impl Session {
    fn read_request(client_stream: &mut UnixStream) -> Result<Request> {
        let mut buf = [0u8; 1024];
        let mut json_bytes = vec![];
        loop {
            let count = client_stream.read(&mut buf)?;
            if count == 0 {
                break Err(Error::InputClosed);
            }
            json_bytes.extend_from_slice(&buf[0..count]);
            match serde_json::from_slice::<Request>(&json_bytes) {
                Ok(request) => break Ok(request),
                Err(err) => {
                    if err.is_eof() {
                        continue;
                    }
                    break Err(crate::Error::Json(err));
                }
            };
        }
    }

    pub fn run(self) -> Result<()> {
        let Self {
            client_count,
            rime_session,
            stop_sender,
            mut client_stream,
        } = self;
        let json_request_processor = JsonRequestProcessor {
            rime_session: &rime_session,
            key_processor: KeyProcessor::new(),
        };
        loop {
            let request = Self::read_request(&mut client_stream);
            let reply = match request {
                Ok(request) => json_request_processor.process_request(request),
                Err(err) => match err.try_into() {
                    Ok(err_outcome) => Reply {
                        id: None,
                        outcome: err_outcome,
                    },
                    // TODO The client can close connection at any point.
                    // Sometimes it's worth logging it.
                    Err(Error::InputClosed) => return Ok(()),
                    Err(err) => {
                        return Err(err);
                    }
                },
            };
            match reply {
                Reply {
                    outcome: Outcome::Effect(Effect::StopClient),
                    ..
                } => {
                    client_stream.write(serde_json::to_string(&reply)?.as_bytes())?;
                    client_stream.flush()?;
                    return Self::check_client_stream_closed(&mut client_stream);
                }
                Reply {
                    outcome: Outcome::Effect(Effect::StopServer),
                    ref id,
                } => {
                    let locked_client_count = client_count.lock().unwrap();
                    if *locked_client_count == 1 {
                        // One string reference is the stop sender here, the other is the one in the
                        // thread that starts threads on `incoming` streams. In such a case, this
                        // is the only thread that is serving a client.
                        client_stream.write(serde_json::to_string(&reply)?.as_bytes())?;
                        client_stream.flush()?;
                        let result = Self::check_client_stream_closed(&mut client_stream);
                        stop_sender.lock().unwrap().send(()).unwrap();
                        return result;
                    } else {
                        client_stream.write(
                            serde_json::to_string(&Reply {
                                id: id.clone(),
                                outcome: Error::MoreThanOneClient.try_into().unwrap(),
                            })?
                            .as_bytes(),
                        )?;
                        client_stream.flush()?;
                    }
                }
                _ => {
                    client_stream.write(serde_json::to_string(&reply)?.as_bytes())?;
                    client_stream.flush()?;
                }
            }
        }
    }

    fn check_client_stream_closed(client_stream: &mut UnixStream) -> Result<()> {
        match Self::read_request(client_stream) {
            Err(Error::InputClosed) => Ok(()),
            Ok(_) => Err(Error::ClientShouldCloseConnection),
            Err(err) => Err(err.into()),
        }
    }
}
