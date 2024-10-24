use crate::json_request_processor::{JsonRequestProcessor, Outcome as ReplyResult, Reply, Request};
use crate::key_processor::KeyProcessor;
use crate::rime_api::RimeSession;
use crate::terminal_interface::TerminalInterface;
use crate::{Args, Call, Effect, Error};
use std::io::{stdout, Write};
use tokio::io::AsyncReadExt;

pub struct JsonStdin {
    stdin: tokio::io::Stdin,
}

impl JsonStdin {
    pub async fn new() -> Self {
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
                break Err(Error::JsonSourceClosed);
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
                result = self.terminal_interface.next_call() => result.map(|call| Request { id: uuid::Uuid::new_v4().into(), call: call }),
                result = self.json_stdin.next_request() => result,
            };
            let reply = match request {
                Ok(Request {
                    id: _,
                    call: Call::Stop,
                }) => {
                    self.terminal_interface.close().await?;
                    break;
                }
                Ok(request) => json_request_processor.process_request(request),
                Err(err) => match err.try_into() {
                    Ok(err_outcome) => Reply {
                        id: None,
                        outcome: err_outcome,
                    },
                    Err(err) => {
                        self.terminal_interface.close().await?;
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
                    outcome:
                        ReplyResult::Effect(Effect::UpdateUi {
                            ref menu,
                            ref composition,
                        }),
                    ..
                } => {
                    writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                    self.terminal_interface.update_ui(composition, menu).await?;
                }
                reply => {
                    writeln!(stdout(), "{}", &serde_json::to_string(&reply)?)?;
                }
            }
        }
        Ok(())
    }
}
