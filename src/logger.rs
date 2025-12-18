use crate::OsLog;
use dashmap::DashMap;
use log::{LevelFilter, Log, Metadata, Record};

pub struct OsLogger {
    loggers: DashMap<String, (Option<LevelFilter>, OsLog)>,
    subsystem: String,
    default_level: LevelFilter,
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let max_level = self
            .loggers
            .get(metadata.target())
            .and_then(|pair| pair.0)
            .unwrap_or(self.default_level);

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
            default_level: LevelFilter::Trace,
        }
    }

    /// Only levels at or above `level` will be logged.
    pub fn level_filter(mut self, level: LevelFilter) -> Self {
        self.default_level = level;
        self.update_global_max_level();
        self
    }

    /// Sets or updates the category's level filter.
    pub fn category_level_filter(self, category: &str, level: LevelFilter) -> Self {
        self.loggers
            .entry(category.into())
            .and_modify(|(existing_level, _)| *existing_level = Some(level))
            .or_insert((Some(level), OsLog::new(&self.subsystem, category)));

        self.update_global_max_level();
        self
    }

    /// Updates the global max level based on the most permissive filter needed.
    fn update_global_max_level(&self) {
        let mut most_permissive = self.default_level;

        // Check all category-specific filters to find the most permissive one
        for entry in self.loggers.iter() {
            if let Some(category_level) = entry.value().0 {
                if category_level > most_permissive {
                    most_permissive = category_level;
                }
            }
        }

        log::set_max_level(most_permissive);
    }

    pub fn init(self) -> Result<(), log::SetLoggerError> {
        // Set the initial global max level
        self.update_global_max_level();
        log::set_boxed_logger(Box::new(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, trace, warn, LevelFilter, Metadata};

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

    #[test]
    fn test_category_filter_more_permissive_than_global() {
        let logger = OsLogger::new("com.example.test")
            .level_filter(LevelFilter::Info) // Global: Info and above
            .category_level_filter("verbose", LevelFilter::Trace); // Category: more permissive

        // Test that global max level is set to most permissive (Trace)
        assert_eq!(log::max_level(), LevelFilter::Trace);

        // Test enabled checks
        let verbose_trace = Metadata::builder()
            .level(log::Level::Trace)
            .target("verbose")
            .build();
        assert!(logger.enabled(&verbose_trace));

        let verbose_debug = Metadata::builder()
            .level(log::Level::Debug)
            .target("verbose")
            .build();
        assert!(logger.enabled(&verbose_debug));

        let default_trace = Metadata::builder()
            .level(log::Level::Trace)
            .target("default_category")
            .build();
        assert!(!logger.enabled(&default_trace)); // Global filter blocks this

        let default_info = Metadata::builder()
            .level(log::Level::Info)
            .target("default_category")
            .build();
        assert!(logger.enabled(&default_info)); // Global filter allows this
    }

    #[test]
    fn test_multiple_category_filters() {
        let logger = OsLogger::new("com.example.test")
            .level_filter(LevelFilter::Warn) // Global: Warn and above
            .category_level_filter("debug_cat", LevelFilter::Debug) // More permissive
            .category_level_filter("quiet_cat", LevelFilter::Error); // Less permissive

        // Global max level should be most permissive (Debug)
        assert_eq!(log::max_level(), LevelFilter::Debug);

        // Test debug category (most permissive)
        let debug_debug = Metadata::builder()
            .level(log::Level::Debug)
            .target("debug_cat")
            .build();
        assert!(logger.enabled(&debug_debug));

        // Test quiet category (less permissive)
        let quiet_warn = Metadata::builder()
            .level(log::Level::Warn)
            .target("quiet_cat")
            .build();
        assert!(!logger.enabled(&quiet_warn)); // Category filter blocks this

        let quiet_error = Metadata::builder()
            .level(log::Level::Error)
            .target("quiet_cat")
            .build();
        assert!(logger.enabled(&quiet_error)); // Category filter allows this
    }

    #[test]
    fn test_dynamic_category_filter_updates() {
        let logger = OsLogger::new("com.example.test").level_filter(LevelFilter::Info);

        // Initially, max level should be Info
        logger.update_global_max_level();
        assert_eq!(log::max_level(), LevelFilter::Info);

        // Add more permissive category filter
        let _logger = logger.category_level_filter("trace_cat", LevelFilter::Trace);

        // Max level should now be Trace
        assert_eq!(log::max_level(), LevelFilter::Trace);
    }
}
