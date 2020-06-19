use crate::OsLog;
use log::{LevelFilter, Log, Metadata, Record};
use dashmap::DashMap;

pub struct OsLogger {
    loggers: DashMap<String, OsLog>,
    level_filter: LevelFilter,
    subsystem: String,
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if metadata.level() > self.level_filter {
            return false;
        }

        let logger = match self.loggers.get(metadata.target()) {
            Some(l) => l,
            None => return false,
        };

        logger.level_is_enabled(metadata.level().into())
    }

    fn log(&self, record: &Record) {
        let message = std::format!("{}", record.args());

        let logger = self
            .loggers
            .entry(record.target().into())
            .or_insert(OsLog::new(&self.subsystem, record.target()));

        logger.with_level(record.level().into(), &message);
    }

    fn flush(&self) {}
}

impl OsLogger {
    /// Creates a new logger. You must also call `init` to finalize the set up.
    /// By default the level filter will be set to `LevelFilter::Trace`.
    pub fn new(subsystem: &str, category: &str) -> Self {
        let loggers = DashMap::new();
        let log = OsLog::new(subsystem, category);
        loggers.insert(category.to_string(), log);

        Self {
            loggers,
            level_filter: LevelFilter::Trace,
            subsystem: subsystem.to_string(),
        }
    }

    /// Modifies the level filter, which by default is `LevelFilter::Trace`.
    /// Only levels at or above `level` will be logged.
    pub fn level_filter(mut self, level: LevelFilter) -> Self {
        self.level_filter = level;
        self
    }

    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_max_level(self.level_filter);
        log::set_boxed_logger(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn};

    #[test]
    fn test_basic_usage() {
        OsLogger::new("com.example.oslog", "testing")
            .level_filter(LevelFilter::Trace)
            .init()
            .unwrap();

        trace!("Trace");
        debug!(target: "Settings", "Debug");
        info!("Info");
        warn!(target: "Database", "Warn");
        error!("Error");
    }
}
