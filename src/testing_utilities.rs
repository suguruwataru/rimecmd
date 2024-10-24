use crate::rime_api::LogLevel;
pub fn temporary_directory_path() -> String {
    format!(
        "temporary_test_directories/user_data_home_{:08X}",
        rand::random::<u32>()
    )
}

/// Log level to be used by all tests.
/// Logging in Rime can only be initilized once per process.
/// Thus, it only makes sense if all tests are initialized
/// with the same parameters.
pub static LOG_LEVEL: LogLevel = LogLevel::OFF;
