mod rime_api;
use rime_api::RimeApi;

mod error;
use error::Error;

mod request_handler;

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
