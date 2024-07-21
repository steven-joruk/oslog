[![Crate](https://img.shields.io/crates/v/oslog.svg)](https://crates.io/crates/oslog)

A minimal wrapper around Apple's unified logging system.

By default support for the [log](https://docs.rs/log) crate is provided, but if
you would prefer just to use the lower level bindings you can disable the
default features.

When making use of targets (`info!(target: "t", "m");`), you should be aware
that a new log is allocated and stored in a map for the lifetime of the program.
Apple also recommends against using dynamically generated subsystems or
categories when using Unified Logging directly, for the same reason.

## Logging example

This is behind the `log` feature flag and is enabled by default.

```rust
fn main() {
    OsLogger::new("com.example.test")
        .level_filter(LevelFilter::Debug)
        .category_level_filter("Settings", LevelFilter::Trace)
        .init()
        .unwrap();

    // Maps to OS_LOG_TYPE_DEBUG
    trace!(target: "Settings", "Trace");

    // Maps to OS_LOG_TYPE_INFO
    debug!("Debug");

    // Maps to OS_LOG_TYPE_DEFAULT
    info!(target: "Parsing", "Info");

    // Maps to OS_LOG_TYPE_ERROR
    warn!("Warn");

    // Maps to OS_LOG_TYPE_FAULT
    error!("Error");
}
```

## Features

`log` enables [log](https://docs.rs/log/latest/log/) crate support via
`OsLogger`.

## Limitations

Most of Apple's logging related functions are macros that enable some
optimizations as well as providing contextual data such as source file location.

By wrapping the macros for use from Rust, we lose those benefits.

Attempting to work around this would involve digging in to opaque types, which
would be an automatic or eventual rejection from the App store.

We also lose performance by having to convert log messages to C strings, even if
there's no observer of in-memory log streams.
