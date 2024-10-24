mod rime_api;
use rime_api::RimeApi;

enum Error<E: std::fmt::Debug> {
    NonUtf8DataHomePath,
    External(E),
}

impl<E: std::fmt::Debug> std::fmt::Debug for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NonUtf8DataHomePath => write!(
                f,
                "data directory path with non-UTF-8 characters is not supported"
            ),
            Error::External(external_error) => external_error.fmt(f),
        }
    }
}

fn main() {
    let data_home = xdg::BaseDirectories::with_prefix("rimed")
        .map_err(|err| Error::External(err))
        .map(|xdg_directories| xdg_directories.get_data_home())
        .unwrap();
    let rime_api = RimeApi::new(data_home, "/usr/share/rime-data", rime_api::LogLevel::INFO);
    println!("{:?}", rime_api.get_user_data_dir());
    println!("{:?}", rime_api.get_shared_data_dir());
    println!("{:?}", rime_api.get_schema_list());
}
