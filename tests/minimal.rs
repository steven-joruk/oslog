use oslog::{Level, OsLog};

#[test]
fn test_subsystem_interior_null() {
    let log = OsLog::new("com.example.oslog\0test", "category");
    log.with_level(Level::Debug, "Hi");
}

#[test]
fn test_category_interior_null() {
    let log = OsLog::new("com.example.oslog", "category\0test");
    log.with_level(Level::Debug, "Hi");
}

#[test]
fn test_message_interior_null() {
    let log = OsLog::new("com.example.oslog", "category");
    log.with_level(Level::Debug, "Hi\0test");
}

#[test]
fn test_message_emoji() {
    let log = OsLog::new("com.example.oslog", "category");
    log.with_level(Level::Debug, "\u{1F601}");
}

#[test]
fn test_global_log_with_level() {
    let log = OsLog::global();
    log.with_level(Level::Debug, "Debug");
    log.with_level(Level::Info, "Info");
    log.with_level(Level::Default, "Default");
    log.with_level(Level::Error, "Error");
    log.with_level(Level::Fault, "Fault");
}

#[test]
fn test_custom_log_with_level() {
    let log = OsLog::new("com.example.oslog", "testing");
    log.with_level(Level::Debug, "Debug");
    log.with_level(Level::Info, "Info");
    log.with_level(Level::Default, "Default");
    log.with_level(Level::Error, "Error");
    log.with_level(Level::Fault, "Fault");
}
