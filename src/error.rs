pub enum Error<E: std::fmt::Debug> {
    NonUtf8DataHomePath,
    NotATerminal,
    UnsupportedInput,
    External(E),
}

impl<E: std::fmt::Debug> std::fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NonUtf8DataHomePath => write!(
                f,
                "data directory path with non-UTF-8 characters is not supported"
            ),
            Error::UnsupportedInput => write!(f, "input is not supported"),
            Error::NotATerminal => write!(f, "not connected to a terminal",),
            Error::External(external_error) => external_error.fmt(f),
        }
    }
}
