mod level;
mod minimal;
mod os_log_string;
mod sys;

#[cfg(feature = "log")]
mod log;

#[cfg(feature = "log")]
pub use log::OsLogger;

pub use level::Level;
pub use minimal::OsLog;
