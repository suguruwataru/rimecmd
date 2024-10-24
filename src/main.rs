#[cfg(test)]
mod testing_utilities;

mod client;
mod error;
mod json_mode;
mod json_request_processor;
mod key_processor;
mod poll_data;
#[allow(dead_code)]
mod rime_api;
mod server_mode;
mod terminal_interface;
mod terminal_json_mode;
mod terminal_mode;
use crate::client::Client;
use crate::server_mode::ServerMode;
use clap::Parser;
use error::Error;
use json_mode::JsonMode;
use rime_api::{RimeComposition, RimeMenu};
use schemars::schema_for;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs::remove_file;
use std::io::{stdout, ErrorKind, Write};
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
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
    StopClient,
    StopServer,
    SchemaName,
    ProcessKey { keycode: usize, mask: usize },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    StopClient,
    StopServer,
    CommitString(String),
    UpdateUi {
        composition: RimeComposition,
        menu: RimeMenu,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum PrintJsonSchemaFor {
    Reply,
    Request,
}

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[arg(long, value_enum, default_value = "none")]
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
    /// Run in server mode.
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
    #[arg(long, short = 'f')]
    /// Start server even if the unix socket already exists.
    ///
    /// Normally, the server creates the unix socket it uses. However, it is
    /// possible that when it tries to create the unix socket, it finds that
    /// a file already exists at the path. This normally means that another
    /// instance of server is running. However, other things might also
    /// cause this to happen, such as that a server has crashed. If you can
    /// be sure that there is not another server instance running, and that
    /// the file at the path does not contain information you want to keep,
    /// use this flag. It will remove the file if it exists and then start
    /// the server as normal.
    force_start_server: bool,
}

#[derive(Clone, Serialize)]
pub struct Config {
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

fn print_json_schema(json_schema: PrintJsonSchemaFor) -> Result<()> {
    match json_schema {
        PrintJsonSchemaFor::Request => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Request))
                    .unwrap()
            )?;
        }
        PrintJsonSchemaFor::Reply => {
            writeln!(
                stdout(),
                "{}",
                serde_json::to_string_pretty(&schema_for!(json_request_processor::Reply)).unwrap()
            )?;
        }
    };
    Ok(())
}

fn start_server() -> Result<()> {
    Command::new(std::env::args().nth(0).unwrap())
        .arg("--server")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

fn rimecmd() -> Result<()> {
    let args = Args::parse();
    if let Some(json_schema) = args.json_schema {
        return print_json_schema(json_schema);
    }
    let config = Config::try_from(&args)?;
    if args.print_config {
        return print_config(config);
    }
    if args.force_start_server {
        remove_file(&config.unix_socket).unwrap_or(());
        return start_server();
    }
    if args.server | args.force_start_server {
        let unix_listener = match UnixListener::bind(&config.unix_socket) {
            Ok(unix_listener) => unix_listener,
            Err(error) => match error.kind() {
                ErrorKind::AddrInUse if args.force_start_server => {
                    UnixListener::bind(&config.unix_socket)?
                }
                ErrorKind::AddrInUse => {
                    return Err(Error::UnixSocketAlreadyExists);
                }
                _ => return Err(error.into()),
            },
        };
        return ServerMode::new(config, unix_listener).main();
    }
    let server_stream = match UnixStream::connect(&config.unix_socket) {
        Ok(server_stream) => server_stream,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => {
                start_server()?;
                loop {
                    match UnixStream::connect(&config.unix_socket) {
                        Ok(server_stream) => break server_stream,
                        Err(error) => match error.kind() {
                            ErrorKind::NotFound => continue,
                            _ => return Err(error.into()),
                        },
                    }
                }
            }
            _ => return Err(error.into()),
        },
    };
    let client = Client::new(server_stream);
    if args.json {
        if args.tty {
            let terminal_interface = terminal_interface::TerminalInterface::new()?;
            return TerminalJsonMode::new(client, terminal_interface).main(args.continue_mode);
        } else {
            return JsonMode::new(client).main(args.continue_mode);
        };
    }
    let maybe_terminal_interface = terminal_interface::TerminalInterface::new();
    match maybe_terminal_interface {
        Ok(terminal_interface) => {
            return TerminalMode::new(client, terminal_interface).main(args.continue_mode)
        }
        Err(Error::NotATerminal) => return JsonMode::new(client).main(args.continue_mode),
        err => {
            err?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match rimecmd() {
        Ok(_) => ExitCode::SUCCESS,
        Err(Error::UnixSocketAlreadyExists) => {
            eprintln!(
                "When the server tries to create a unix socket to listen to, \
                it finds that there already exists one."
            );
            eprintln!("This usually means that a server is already running.");
            eprintln!(
                "If you are sure that there does not exist \
                an already running server instance, use `--force-start-server`."
            );
            eprintln!(
                "This will remove the already existing unix socket and then start the server."
            );
            ExitCode::from(22)
        }
        Err(err) => {
            eprintln!("{:?}", err);
            ExitCode::FAILURE
        }
    }
}
