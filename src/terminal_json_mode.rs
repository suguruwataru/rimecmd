use crate::json_request_processor::{JsonRequestProcessor, Reply, Request, Result as ReplyResult};
use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::{Action, Args, Call, Error};
use std::io::{stdout, Write};
use tokio::io::AsyncReadExt;

pub struct JsonStdin {
    stdin: tokio::io::Stdin,
}

impl JsonStdin {
    pub fn new() -> Self {
        Self {
            stdin: tokio::io::stdin(),
        }
    }

    pub async fn next_request(&mut self) -> Result<Request, Error> {
        let mut buf = [0u8; 1024];
        let mut json_bytes = vec![];
        loop {
            let count = self.stdin.read(&mut buf).await?;
            if count == 0 {
                break Err(Error::UnsupportedInput);
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
    pub args: Args,
    pub terminal_interface: TerminalInterface,
    pub json_stdin: JsonStdin,
    pub rime_session: RimeSession<'a>,
}

impl TerminalJsonMode<'_> {
    pub async fn main(&mut self) -> Result<(), Error> {
        let json_request_processor = JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: KeyProcessor::new(),
        };
        self.terminal_interface.open().await?;
        loop {
            let request = tokio::select! {
                call = self.terminal_interface.next_call() => Request { id: uuid::Uuid::new_v4().into(), call: call? },
                request = self.json_stdin.next_request() => request?,
            };
            let reply = match request {
                Request {
                    id: _,
                    call: Call::Stop,
                } => {
                    self.terminal_interface.close().await?;
                    break;
                }
                request => json_request_processor.process_request(request),
            };
            match reply {
                Reply {
                    result: ReplyResult::Action(Action::CommitString(_)),
                    ..
                } => {
                    if !self.args.continue_mode {
                        self.terminal_interface.close().await?;
                        writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                        break;
                    } else {
                        self.terminal_interface.remove_ui().await?;
                        writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                        self.terminal_interface.setup_ui().await?;
                    }
                }
                Reply {
                    result: ReplyResult::Action(Action::UpdateUi { menu, composition }),
                    ..
                } => {
                    self.terminal_interface.update_ui(composition, menu).await?;
                }
                _ => todo!(),
            }
        }
        Ok(())
    }
}
