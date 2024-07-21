use crate::{os_log_string::OsLogString, sys::wrapped_os_log_type_enabled, OsLog};
use log::{LevelFilter, Log, Metadata, Record};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{CStr, CString},
    hash::{DefaultHasher, Hash, Hasher},
};

thread_local! {
    static LOGS: RefCell<HashMap<LogKey, (LevelFilter, OsLog)>> = Default::default();
}

pub struct OsLogger {
    subsystem: CString,
}

impl Log for OsLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let category = metadata.target();
        let key = get_log_key(self.subsystem.as_c_str(), metadata.target());

        LOGS.with_borrow_mut(|logs| {
            let (max_level, log) = logs
                .entry(key)
                .or_insert_with(|| (log::max_level(), OsLog::new(&self.subsystem, category)));

            let ffi_level = match max_level {
                log::LevelFilter::Off => return false,
                log::LevelFilter::Trace => crate::Level::Debug,
                log::LevelFilter::Debug => crate::Level::Info,
                log::LevelFilter::Info => crate::Level::Default,
                log::LevelFilter::Warn => crate::Level::Error,
                log::LevelFilter::Error => crate::Level::Fault,
            } as u8;

            let enabled = unsafe { wrapped_os_log_type_enabled(log.inner, ffi_level) };
            if !enabled {
                return false;
            }

            metadata.level() <= *max_level
        })
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let key = get_log_key(self.subsystem.as_c_str(), record.metadata().target());

        LOGS.with_borrow_mut(|logs| {
            if let Some((_, log)) = logs.get(&key) {
                let message = std::format!("{}", record.args());
                log.with_level(record.level().into(), &message);
            }
        });
    }

    fn flush(&self) {}
}

impl OsLogger {
    /// Creates a new logger. You must also call `init` to finalize the set up.
    /// By default, the level filter will be set to `LevelFilter::Trace`.
    pub fn new<S: OsLogString + ?Sized>(subsystem: &S) -> Self {
        subsystem.with_cstr(|s| Self {
            subsystem: s.to_owned(),
        })
    }

    /// Only levels at or above `level` will be logged.
    pub fn level_filter(self, level: LevelFilter) -> Self {
        log::set_max_level(level);
        self
    }

    /// Sets or updates the category's level filter.
    pub fn category_level_filter(self, category: &str, new_level: LevelFilter) -> Self {
        let key = get_log_key(self.subsystem.as_c_str(), category);

        LOGS.with_borrow_mut(|logs| {
            logs.entry(key)
                .and_modify(|(level, _)| *level = new_level)
                .or_insert_with(|| (new_level, OsLog::new(&self.subsystem, category)));
        });

        self
    }

    pub fn init(self) -> Result<(), log::SetLoggerError> {
        log::set_boxed_logger(Box::new(self))
    }
}

/// This is a hash of catgeory + subsystem
type LogKey = u64;

fn get_log_key(subsystem: &CStr, category: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    subsystem.hash(&mut hasher);
    category.hash(&mut hasher);
    hasher.finish()
}
