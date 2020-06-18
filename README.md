A minimal wrapper around Apple's unified logging system.

By default support for the [log](https://docs.rs/log) crate is provided, but you can disable it using the feature flags if you like:

```toml
[dependencies]
oslog = { version = "0.0.2", default-features = false }
```

# Example

```rust
fn main() {
    OsLogger::new("com.example.test", "testing")
        .level_filter(LevelFilter::Debug)
        .init()
        .unwrap();

    // Maps to OS_LOG_TYPE_DEBUG
    trace!("Trace");

    // Maps to OS_LOG_TYPE_INFO
    debug!("Debug");

    // Maps to OS_LOG_TYPE_DEFAULT
    info!("Info");

    // Maps to OS_LOG_TYPE_ERROR
    warn!("Warn");

    // Maps to OS_LOG_TYPE_FAULT
    error!("Error");
}
```

# Missing features

Almost everything :).

* Multiple categories, although I'm planning on adding optional support for setting the category to the module name which invoked the log call.
* Activities
* Tracing
* Native support for line numbers and file names.
