#[cfg(test)]
mod testing_utilities;

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
use std::path::PathBuf;
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

use clap::Parser;
use schemars::schema_for;
use std::io::{stdout, Read, Write};
use std::os::unix::net::UnixListener;
use std::process::ExitCode;
use std::thread;

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
    #[arg(long, requires = "json", value_name = "PATH")]
    unix_socket: Option<PathBuf>,
    #[arg(long)]
    server: bool,
}

fn rimecmd() -> Result<()> {
    let args = Args::parse();
    match args.json_schema {
        Some(PrintJsonSchemaFor::Request) => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Request))
                    .unwrap()
            )?;
        }
        Some(PrintJsonSchemaFor::Reply) => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Reply)).unwrap()
            )?;
        }
        None => (),
    }
    let xdg_directories = xdg::BaseDirectories::with_prefix("rimecmd")?;
    let data_home = xdg_directories.get_data_home();
    let rime_api = rime_api::RimeApi::new(&data_home, "/usr/share/rime-data", args.rime_log_level);
    let rime_session = rime_api::RimeSession::new(&rime_api);
    let socket_path = if let Some(ref socket_path) = args.unix_socket {
        PathBuf::from(socket_path)
    } else {
        if let Ok(runtime_directory) = xdg_directories.create_runtime_directory("socket") {
            runtime_directory
        } else {
            std::env::temp_dir()
        }
        .join("rimecmd.sock")
    };
    if args.server {
        let listener = UnixListener::bind(&socket_path).unwrap();
        for stream in listener.incoming() {
            let mut stream = stream?;
            thread::spawn(move || loop {
                let mut buf = [0; 1024];
                let count = match stream.read(&mut buf) {
                    Ok(count) => count,
                    Err(err) => return Error::from(err),
                };
                if count == 0 {
                    return Error::InputClosed;
                }
                println!("{:?}", &buf[0..count]);
            });
        }
        return Ok(());
    }
    if args.json {
        let json_stdin = JsonStdin::new();
        if args.tty {
            let terminal_interface = terminal_interface::TerminalInterface::new()?;
            TerminalJsonMode::new(args, terminal_interface, json_stdin, rime_session).main()?;
            return Ok(());
        } else {
            JsonMode::new(args, json_stdin, rime_session).main()?;
            return Ok(());
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
            .main()?;
        }
        Err(Error::NotATerminal) => {
            JsonMode::new(args, JsonStdin::new(), rime_session).main()?;
        }
        err => {
            err?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match rimecmd() {
        Ok(_) => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
