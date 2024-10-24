#[cfg(test)]
mod testing_utilities;

mod error;
mod json_mode;
mod json_request_processor;
mod json_source;
mod key_processor;
mod poll_data;
#[allow(dead_code)]
mod rime_api;
mod server_mode;
mod terminal_interface;
mod terminal_json_mode;
mod terminal_mode;

use error::Error;
use json_mode::JsonMode;
use json_source::JsonSource;
use rime_api::{RimeComposition, RimeMenu};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use server_mode::ServerMode;
use std::path::PathBuf;
use std::sync::{mpsc::channel, Arc, Mutex};
use terminal_json_mode::TerminalJsonMode;
use terminal_mode::TerminalMode;

type Result<T> = std::result::Result<T, Error>;

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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    CommitString(String),
    UpdateUi {
        composition: RimeComposition,
        menu: RimeMenu,
    },
    Stop,
}

use clap::Parser;
use schemars::schema_for;
use std::io::{stdout, Write};
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
    /// Path for the Unix socket to connect to.
    ///
    /// When used in server mode, this is used to determine the Unix socket to
    /// listen to. Otherwise, this decides the Unix socket to be used to connect
    /// to the server. When absent, this is determined based on the environment
    /// and XDG specification. Use `--print-config` to see the value.
    unix_socket: Option<PathBuf>,
    #[arg(long)]
    /// Run `rimecmd` in server mode.
    ///
    /// When Rime, on which `rimecmd` is based, runs, it manipulates data in the
    /// user data directory (see `--print-confg`). It is not a good idea to
    /// have multiple process mutate the same piece of data. As a result, `rimecmd`
    /// is based on a client-server architecture. Only the server accesses the Rime
    /// API which mutates the data, and only one instance of server runs at a time.
    /// Clients connect to the server to communicate with Rime.
    server: bool,
    /// Print the configuration used by `rimecmd`.
    ///
    /// The output is in JSON format.
    #[arg(long)]
    print_config: bool,
}

#[derive(Clone, Serialize)]
pub struct Config {
    pub continue_mode: bool,
    pub unix_socket: PathBuf,
    pub user_data_directory: PathBuf,
    pub rime_log_level: rime_api::LogLevel,
}

impl TryFrom<&Args> for Config {
    type Error = Error;
    fn try_from(args: &Args) -> Result<Self> {
        let xdg_directories = xdg::BaseDirectories::with_prefix("rimecmd")?;
        let unix_socket = if let Some(ref socket_path) = args.unix_socket {
            PathBuf::from(socket_path)
        } else {
            if let Ok(runtime_directory) = xdg_directories.create_runtime_directory("socket") {
                runtime_directory
            } else {
                std::env::temp_dir()
            }
            .join("rimecmd.sock")
        };
        Ok(Self {
            continue_mode: args.continue_mode,
            unix_socket,
            user_data_directory: xdg_directories.get_data_home().into(),
            rime_log_level: args.rime_log_level,
        })
    }
}

fn print_config(config: Config) -> Result<()> {
    writeln!(stdout(), "{}", serde_json::to_string_pretty(&config)?)?;
    Ok(())
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
    let config = Config::try_from(&args)?;
    if args.print_config {
        return print_config(config);
    }
    if args.server {
        let listener = UnixListener::bind(&config.unix_socket).unwrap();
        let (error_sender, error_receiver) = channel();
        let error_sender = Arc::new(Mutex::new(error_sender));
        thread::spawn(move || loop {
            let _error: Error = error_receiver.recv().unwrap();
            todo!("implement error logging");
        });
        for stream in listener.incoming() {
            let stream = stream?;
            let config = config.clone();
            let error_sender = Arc::clone(&error_sender);
            thread::spawn(move || {
                let rime_api = rime_api::RimeApi::new(
                    &config.user_data_directory,
                    "/usr/share/rime-data",
                    config.rime_log_level,
                );
                let rime_session = rime_api::RimeSession::new(&rime_api);
                let input_stream = match stream.try_clone() {
                    Ok(stream) => stream,
                    Err(error) => {
                        error_sender.lock().unwrap().send(error.into()).unwrap();
                        return ();
                    }
                };
                ServerMode {
                    json_source: JsonSource::new(input_stream),
                    json_dest: stream,
                    rime_session,
                }
                .main()
                .unwrap_or_else(|err| error_sender.lock().unwrap().send(err).unwrap());
            });
        }
        return Ok(());
    }
    let rime_api = rime_api::RimeApi::new(
        &config.user_data_directory,
        "/usr/share/rime-data",
        args.rime_log_level,
    );
    let rime_session = rime_api::RimeSession::new(&rime_api);
    if args.json {
        let json_source = JsonSource::new(std::io::stdin());
        let json_dest = std::io::stdout();
        if args.tty {
            let terminal_interface = terminal_interface::TerminalInterface::new()?;
            TerminalJsonMode::new(config, terminal_interface).main()?;
            return Ok(());
        } else {
            ServerMode {
                json_source,
                json_dest,
                rime_session,
            }
            .main()?;
            return Ok(());
        };
    }
    let maybe_terminal_interface = terminal_interface::TerminalInterface::new();
    match maybe_terminal_interface {
        Ok(terminal_interface) => {
            TerminalMode {
                config,
                terminal_interface,
                rime_session,
            }
            .main()?;
        }
        Err(Error::NotATerminal) => {
            JsonMode {
                config,
                rime_session,
            }
            .main()?;
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
        Err(err) => {
            eprintln!("{:?}", err);
            ExitCode::FAILURE
        }
    }
}
