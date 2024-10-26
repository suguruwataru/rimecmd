pub enum Error {
    NonUtf8DataHomePath,
    NotATerminal,
    UnsupportedInput,
    ClientShouldCloseConnection,
    MoreThanOneClient,
    ServerClosedConnection,
    UnixSocketAlreadyExists,
    ConfigNotFound(String),
    OptionNotFound(String),
    OneOfMultipleInputClosed,
    Io(std::io::Error),
    Json(serde_json::Error),
    Xdg(xdg::BaseDirectoriesError),
    NulInCString(std::ffi::NulError),
}

impl From<&Error> for std::process::ExitCode {
    fn from(error: &Error) -> Self {
        use Error::*;
        match error {
            OneOfMultipleInputClosed => Self::from(2),
            UnsupportedInput => Self::from(3),
            UnixSocketAlreadyExists => Self::from(4),
            MoreThanOneClient => Self::from(5),
            _ => Self::FAILURE,
        }
    }
}

impl From<Error> for std::process::ExitCode {
    fn from(error: Error) -> Self {
        (&error).into()
    }
}

impl From<std::io::Error> for crate::Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<serde_json::Error> for crate::Error {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

impl From<xdg::BaseDirectoriesError> for crate::Error {
    fn from(source: xdg::BaseDirectoriesError) -> Self {
        Self::Xdg(source)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConfigNotFound(config_name) => {
                write!(f, "the config {} is not found", config_name)
            }
            Error::OptionNotFound(option_name) => {
                write!(f, "the config option {} is not found", option_name,)
            }
            Error::NonUtf8DataHomePath => write!(
                f,
                "data directory path with non-UTF-8 characters is not supported"
            ),
            Error::UnsupportedInput => write!(f, "input is not supported"),
            Error::NotATerminal => write!(f, "not connected to a terminal"),
            Error::OneOfMultipleInputClosed => write!(f, "one of data sources is closed"),
            Error::ServerClosedConnection => write!(f, "server closed connection"),
            Error::ClientShouldCloseConnection => {
                write!(f, "client should have closed connection, but it didn't")
            }
            Error::UnixSocketAlreadyExists => {
                write!(f, "the unix socket to use already exists")
            }
            Error::MoreThanOneClient => {
                write!(f, "there are other clients, so server cannot stop")
            }
            Error::Io(io_err) => io_err.fmt(f),
            Error::Json(json_err) => json_err.fmt(f),
            Error::NulInCString(nul_err) => nul_err.fmt(f),
            Error::Xdg(xdg_error) => xdg_error.fmt(f),
        }
    }
}
