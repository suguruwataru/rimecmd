mod error;
mod json_mode;
mod json_request_processor;
mod json_stdin;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
mod terminal_json_mode;
mod terminal_mode;
use error::Error;
type Result<T> = std::result::Result<T, Error>;

use json_mode::JsonMode;
use json_stdin::JsonStdin;
use rime_api::{RimeComposition, RimeMenu};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terminal_json_mode::TerminalJsonMode;
use terminal_mode::TerminalMode;
mod poll_request;

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

#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
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

fn main() -> ExitCode {
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
    if args.json {
        let json_stdin = JsonStdin::new();
        if args.tty {
            let maybe_terminal_interface = terminal_interface::TerminalInterface::new();
            return match maybe_terminal_interface {
                Ok(terminal_interface) => {
                    TerminalJsonMode::new(args, terminal_interface, json_stdin, rime_session)
                        .main()
                        .unwrap();
                    ExitCode::SUCCESS
                }
                Err(_) => ExitCode::FAILURE,
            };
        } else {
            JsonMode::new(args, json_stdin, rime_session)
                .main()
                .unwrap();
            return ExitCode::SUCCESS;
        };
    }
    let maybe_terminal_interface = terminal_interface::TerminalInterface::new();
    match maybe_terminal_interface {
        Ok(terminal_interface) => {
            TerminalMode {
                args,
                terminal_interface,
                rime_session,
            }
            .main()
            .unwrap();
            ExitCode::SUCCESS
        }
        Err(Error::NotATerminal) => {
            JsonMode::new(args, JsonStdin::new(), rime_session)
                .main()
                .unwrap();
            return ExitCode::SUCCESS;
        }
        Err(_) => ExitCode::FAILURE,
    }
}
