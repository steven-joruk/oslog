use crate::OsLog;
use dashmap::DashMap;
use derivative::Derivative;
use log::{LevelFilter, Log, Metadata, Record};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct OsLogger {
    #[derivative(Debug = "ignore")]
    loggers: DashMap<String, (Option<LevelFilter>, OsLog)>,
    subsystem: String,
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let max_level = self
            .loggers
            .get(metadata.target())
            .and_then(|pair| pair.0)
            .unwrap_or_else(log::max_level);

        metadata.level() <= max_level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let pair = self
                .loggers
                .entry(record.target().into())
                .or_insert((None, OsLog::new(&self.subsystem, record.target())));

            let message = std::format!("{}", record.args());
            pair.1.with_level(record.level().into(), &message);
        }
    }

    fn flush(&self) {}
}

impl OsLogger {
    /// Creates a new logger. You must also call `init` to finalize the set up.
    /// By default the level filter will be set to `LevelFilter::Trace`.
    pub fn new(subsystem: &str) -> Self {
        Self {
            loggers: DashMap::new(),
            subsystem: subsystem.to_string(),
        }
    }

    /// Only levels at or above `level` will be logged.
    pub fn level_filter(self, level: LevelFilter) -> Self {
        log::set_max_level(level);
        self
    }

    /// Sets or updates the category's level filter.
    pub fn category_level_filter(self, category: &str, level: LevelFilter) -> Self {
        self.loggers
            .entry(category.into())
            .and_modify(|(existing_level, _)| *existing_level = Some(level))
            .or_insert((Some(level), OsLog::new(&self.subsystem, category)));

        self
    }

    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn};
    use log4rs::append::console::ConsoleAppender;
    use log4rs::config::{Appender, Root};
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::Config;

    #[test]
    fn test_log4rs() {
        let os_logger = OsLogger::new("com.example.oslog")
            .level_filter(LevelFilter::Trace)
            .category_level_filter("Settings", LevelFilter::Warn)
            .category_level_filter("Database", LevelFilter::Error)
            .category_level_filter("Database", LevelFilter::Trace);

        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{m}{n}")))
            .build();
        match Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .appender(Appender::builder().build("os_logger", Box::new(os_logger)))
            .build(
                Root::builder()
                    .appender("stdout")
                    .appender("os_logger")
                    .build(LevelFilter::Debug),
            ) {
            Ok(config) => {
                let handle = log4rs::init_config(config);
                if let Err(e) = handle {
                    println!("ERROR: failed to configure logging for stdout with {e:?}");
                }
            }
            Err(e) => {
                println!("ERROR: failed to prepare default logging configuration with {e:?}");
            }
        }
        // This will not be logged to the Console because of its category's custom level filter.
        info!(target: "Settings", "Info");

        warn!(target: "Settings", "Warn");
        error!(target: "Settings", "Error");

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!(target: "Database", "Warn");
        error!("Error");
    }

    #[test]
    fn test_basic_usage() {
        OsLogger::new("com.example.oslog")
            .level_filter(LevelFilter::Trace)
            .category_level_filter("Settings", LevelFilter::Warn)
            .category_level_filter("Database", LevelFilter::Error)
            .category_level_filter("Database", LevelFilter::Trace)
            .init()
            .unwrap();

        // This will not be logged because of its category's custom level filter.
        info!(target: "Settings", "Info");

        warn!(target: "Settings", "Warn");
        error!(target: "Settings", "Error");

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!(target: "Database", "Warn");
        error!("Error");
    }
}
