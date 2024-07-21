use log::{debug, error, info, trace, warn, LevelFilter};
use oslog::OsLogger;

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
