mod error;
#[allow(dead_code)]
mod json_request_processor;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
use error::Error;
use json_request_processor::{Call, Reply, Result};
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
    /// stdin and stdout is used, and input/output using the terminal is turned
    /// off.
    json: bool,
    #[arg(long)]
    /// Use the terminal for interaction.
    ///
    /// This is the default behavior when this program is run on a terminal.
    /// However, when JSON IO is used, by default, terminal IO will be turned
    /// off. This switch lets this program also use terminal IO even in this
    /// case.
    tty: bool,
    #[arg(short, long = "continue")]
    /// Do not exit after committing once, instead, continue to process input.
    continue_mode: bool,
    #[arg(short, long)]
    /// Print the JSON schema used, then exit.
    json_schema: Option<PrintJsonSchemaFor>,
}

async fn next_call_from_stdin(
    stdin: &mut tokio::io::Stdin,
) -> std::result::Result<Call, crate::Error> {
    let mut buf = [0u8; 1024];
    let mut json_bytes = vec![];
    loop {
        let count = stdin.read(&mut buf).await?;
        if count == 0 {
            break Err(Error::UnsupportedInput);
        }
        json_bytes.extend_from_slice(&buf[0..count]);
        match serde_json::from_slice::<Call>(&json_bytes) {
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
    let maybe_terminal_interface = terminal_interface::TerminalInterface::new().await;
    let key_processor = key_processor::KeyProcessor::new();
    let mut stdin = tokio::io::stdin();
    match maybe_terminal_interface {
        Ok(mut terminal_interface) => {
            terminal_interface.open().await.unwrap();
            loop {
                let call = if args.json {
                    tokio::select! {
                        call = next_call_from_stdin(&mut stdin) => call,
                        call = terminal_interface.next_call() => call,
                    }
                } else {
                    terminal_interface.next_call().await
                }
                .unwrap();
                if let Call::Stop = call {
                    break;
                }
                let action = match call {
                    Call::ProcessKey { keycode, mask } => {
                        key_processor.process_key(&rime_session, keycode, mask)
                    }
                    Call::Stop => {
                        terminal_interface.close().await.unwrap();
                        break;
                    }
                    Call::SchemaName => {
                        stdout()
                            .write(
                                serde_json::to_string(&Reply {
                                    id: Some("22".into()),
                                    result: Result::SchemaName(rime_session.get_current_schema()),
                                })
                                .unwrap()
                                .as_bytes(),
                            )
                            .unwrap();
                        stdout().flush().unwrap();
                        continue;
                    }
                };
                if args.json {
                    writeln!(
                        stdout(),
                        "{}",
                        serde_json::to_string(&Reply {
                            id: None,
                            result: Result::Action(action.clone())
                        })
                        .unwrap()
                    )
                    .unwrap();
                    stdout().flush().unwrap();
                }
                match action {
                    Action::CommitString(commit_string) => {
                        if !args.continue_mode {
                            terminal_interface.close().await.unwrap();
                            if !args.json {
                                writeln!(stdout(), "{}", commit_string).unwrap();
                                stdout().flush().unwrap();
                            }
                            break;
                        } else {
                            terminal_interface.remove_ui().await.unwrap();
                            if !args.json {
                                writeln!(stdout(), "{}", commit_string).unwrap();
                                stdout().flush().unwrap();
                            }
                            terminal_interface.setup_ui().await.unwrap();
                        }
                    }
                    Action::UpdateUi { menu, composition } => {
                        terminal_interface
                            .update_ui(composition, menu)
                            .await
                            .unwrap();
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(Error::NotATerminal) => todo!(),
        Err(_) => ExitCode::FAILURE,
    }
}
