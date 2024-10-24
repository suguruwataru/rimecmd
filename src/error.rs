pub enum Error {
    NonUtf8DataHomePath,
    NotATerminal,
    UnsupportedInput,
    ServerClosedConnection,
    InputClosed,
    Io(std::io::Error),
    Json(serde_json::Error),
    Xdg(xdg::BaseDirectoriesError),
    NulInCString(std::ffi::NulError),
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
            Error::NonUtf8DataHomePath => write!(
                f,
                "data directory path with non-UTF-8 characters is not supported"
            ),
            Error::UnsupportedInput => write!(f, "input is not supported"),
            Error::NotATerminal => write!(f, "not connected to a terminal"),
            Error::InputClosed => write!(f, "one of data sources is closed"),
            Error::ServerClosedConnection => write!(f, "server closed connection"),
            Error::Io(io_err) => io_err.fmt(f),
            Error::Json(json_err) => json_err.fmt(f),
            Error::NulInCString(nul_err) => nul_err.fmt(f),
            Error::Xdg(xdg_error) => xdg_error.fmt(f),
        }
    }
}
