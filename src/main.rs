mod error;
#[allow(dead_code)]
mod json_request_processor;
mod key_processor;
#[allow(dead_code)]
mod rime_api;
mod terminal_interface;
use error::Error;

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
struct Args {
    #[arg(long, short = 'l', value_enum, default_value = "none")]
    /// The lowest level of Rime logs to write to stderr.
    ///
    /// When `none`, no logs will be written.
    rime_log_level: rime_api::LogLevel,
    #[arg(long)]
    /// Use JSON for input/output.
    ///
    /// When not indicated otherwise, stdio is used.
    json: bool,
    /// Interact through stdio instead of the terminal
    ///
    /// This is the default when not run from a terminal.
    /// Implies --json.
    #[arg(long)]
    stdio: bool,
    #[arg(short, long = "continue")]
    /// Do not exit after committing once, instead, continue to process input.
    continue_mode: bool,
    #[arg(short, long)]
    /// Print the JSON schema used, then exit.
    json_schema: Option<PrintJsonSchemaFor>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match args.json_schema {
        Some(PrintJsonSchemaFor::Request) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Request))
                    .unwrap()
            );
            return ExitCode::SUCCESS;
        }
        Some(PrintJsonSchemaFor::Reply) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Reply)).unwrap()
            );
            return ExitCode::SUCCESS;
        }
        None => (),
    }
    let data_home = xdg::BaseDirectories::with_prefix("rimecmd")
        .map_err(|err| Error::External(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = rime_api::RimeApi::new(&data_home, "/usr/share/rime-data", args.rime_log_level);
    if args.json || args.stdio {
        todo!()
    } else {
        let maybe_terminal_interface = terminal_interface::TerminalInterface::new(
            key_processor::KeyProcessor::new(rime_api::RimeSession::new(&rime_api)),
        );
        match maybe_terminal_interface {
            Ok(mut terminal_interface) => {
                if let Some(commit_string) = terminal_interface.process_input().unwrap() {
                    writeln!(stdout(), "{}", commit_string).unwrap();
                }
                ExitCode::SUCCESS
            }
            Err(Error::NotATerminal) => todo!(),
            Err(_) => ExitCode::FAILURE,
        }
    }
}
