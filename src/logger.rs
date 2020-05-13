use log::{LevelFilter, Log, Metadata, Record};
use crate::OsLog;

pub struct OsLogger {
    logger: OsLog,
    level_filter: LevelFilter,
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if metadata.level() > self.level_filter {
            return false;
        }

        self.logger.level_is_enabled(metadata.level().into())
    }

    fn log(&self, record: &Record) {
        let message = std::format!("{}", record.args());
        self.logger.with_level(record.level().into(), &message);
    }

    fn flush(&self) {}
}

impl OsLogger {
    /// Creates a new logger. You must also call `init` to finalize the set up.
    /// By default the level filter will be set to `LevelFilter::Trace`.
    pub fn new(subsystem: &str, category: &str) -> Self {
        Self {
            logger: OsLog::new(subsystem, category),
            level_filter: LevelFilter::Trace,
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
        OsLogger::new("com.example.test", "testing")
            .level_filter(LevelFilter::Debug)
            .init()
            .unwrap();

        trace!("Trace");
        debug!("Debug");
        info!("Info");
        warn!("Warn");
        error!("Error");
    }
}
