mod error;
#[allow(dead_code)]
mod json_request_processor;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
use error::Error;
use json_request_processor::{Call, Reply, Request, Result as ReplyResult};
use key_processor::Action;

#[cfg(test)]
mod testing_utilities;

use clap::Parser;
use schemars::schema_for;
use std::io::{stdout, Write};
use std::process::ExitCode;

use tokio::io::AsyncReadExt;

#[derive(Clone, clap::ValueEnum)]
enum PrintJsonSchemaFor {
    Reply,
    Request,
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
    #[arg(long, short = 'l', value_enum, default_value = "none")]
    /// The lowest level of Rime logs to write to stderr.
    ///
    /// When `none`, no logs will be written.
    rime_log_level: rime_api::LogLevel,
    #[arg(long)]
    /// Use JSON for input/output.
    ///
    /// This is the default behavior
    /// stdin and stdout is used, and input/output using the terminal is turned
    /// off.
    json: bool,
    #[arg(long)]
    /// Use the terminal for interaction.
    ///
    /// This is the default behavior when this program is run on a terminal.
    /// However, even in this case, when JSON is used, by default, terminal
    /// interaction will be turned off. This switch lets this program also
    /// use terminal interaction even in this case.
    tty: bool,
    #[arg(short, long = "continue")]
    /// Do not exit after committing once, instead, continue to process input.
    continue_mode: bool,
    #[arg(short, long, exclusive(true))]
    /// Print the JSON schema used, then exit.
    json_schema: Option<PrintJsonSchemaFor>,
}

use terminal_interface::TerminalInterface;

struct TerminalMode<'a> {
    pub args: Args,
    pub terminal_interface: TerminalInterface,
    pub rime_session: RimeSession<'a>,
}

use rime_api::RimeSession;

impl<'a> TerminalMode<'a> {
    pub async fn main(mut self) -> Result<(), Error> {
        let key_processor = key_processor::KeyProcessor::new();
        self.terminal_interface.open().await?;
        loop {
            let call = self.terminal_interface.next_call().await?;
            let action = match call {
                Call::ProcessKey { keycode, mask } => {
                    key_processor.process_key(&self.rime_session, keycode, mask)
                }
                Call::Stop => {
                    self.terminal_interface.close().await?;
                    break;
                }
                _ => unreachable!(),
            };
            match action {
                Action::CommitString(commit_string) => {
                    if !self.args.continue_mode {
                        self.terminal_interface.close().await?;
                        writeln!(stdout(), "{}", commit_string)?;
                        break;
                    } else {
                        self.terminal_interface.remove_ui().await?;
                        writeln!(stdout(), "{}", commit_string)?;
                        self.terminal_interface.setup_ui().await?;
                    }
                }
                Action::UpdateUi { menu, composition } => {
                    self.terminal_interface.update_ui(composition, menu).await?;
                }
            }
        }
        Ok(())
    }
}

struct TerminalJsonMode<'a> {
    pub args: Args,
    pub terminal_interface: TerminalInterface,
    pub json_stdin: JsonStdin,
    pub rime_session: RimeSession<'a>,
}

impl TerminalJsonMode<'_> {
    pub async fn main(&mut self) -> Result<(), Error> {
        let json_request_processor = json_request_processor::JsonRequestProcessor {
            rime_session: &self.rime_session,
            key_processor: key_processor::KeyProcessor::new(),
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

struct JsonStdin {
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();
    match args.json_schema {
        Some(PrintJsonSchemaFor::Request) => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Request))
                    .unwrap()
            )
            .unwrap();
            return ExitCode::SUCCESS;
        }
        Some(PrintJsonSchemaFor::Reply) => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Reply)).unwrap()
            )
            .unwrap();
            return ExitCode::SUCCESS;
        }
        None => (),
    }
    let data_home = xdg::BaseDirectories::with_prefix("rimecmd")
        .map_err(|err| Error::Xdg(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = rime_api::RimeApi::new(&data_home, "/usr/share/rime-data", args.rime_log_level);
    let rime_session = rime_api::RimeSession::new(&rime_api);
    if args.json && args.tty {
        let maybe_terminal_interface = terminal_interface::TerminalInterface::new().await;
        let json_stdin = JsonStdin::new();
        return match maybe_terminal_interface {
            Ok(terminal_interface) => {
                TerminalJsonMode {
                    args,
                    terminal_interface,
                    json_stdin,
                    rime_session,
                }
                .main()
                .await
                .unwrap();
                ExitCode::SUCCESS
            }
            Err(_) => ExitCode::FAILURE,
        };
    }
    let maybe_terminal_interface = terminal_interface::TerminalInterface::new().await;
    match maybe_terminal_interface {
        Ok(terminal_interface) => {
            TerminalMode {
                args,
                terminal_interface,
                rime_session,
            }
            .main()
            .await
            .unwrap();
            ExitCode::SUCCESS
        }
        Err(Error::NotATerminal) => todo!(),
        Err(_) => ExitCode::FAILURE,
    }
}
