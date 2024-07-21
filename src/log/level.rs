impl From<log::Level> for crate::Level {
    fn from(other: log::Level) -> Self {
        match other {
            log::Level::Trace => Self::Debug,
            log::Level::Debug => Self::Info,
            log::Level::Info => Self::Default,
            log::Level::Warn => Self::Error,
            log::Level::Error => Self::Fault,
        }
    }
}
