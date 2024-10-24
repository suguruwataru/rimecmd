mod error;
mod json_request_processor;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
mod terminal_json_mode;
mod terminal_mode;
use error::Error;
use rime_api::{RimeComposition, RimeMenu};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terminal_json_mode::{JsonStdin, TerminalJsonMode};
use terminal_mode::TerminalMode;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    tag = "method",
    content = "params",
    deny_unknown_fields
)]
pub enum Call {
    SchemaName,
    Stop,
    ProcessKey { keycode: usize, mask: usize },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    rename_all = "snake_case",
    tag = "action",
    content = "params",
    deny_unknown_fields
)]
pub enum Action {
    CommitString(String),
    UpdateUi {
        composition: RimeComposition,
        menu: RimeMenu,
    },
}

#[cfg(test)]
mod testing_utilities;

use clap::Parser;
use schemars::schema_for;
use std::io::{stdout, Write};
use std::process::ExitCode;

#[derive(Clone, clap::ValueEnum)]
enum PrintJsonSchemaFor {
    Reply,
    Request,
}

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
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
        let json_stdin = JsonStdin::new().await;
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
