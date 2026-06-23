[![Crate](https://img.shields.io/crates/v/oslog.svg)](https://crates.io/crates/oslog)

A minimal safe wrapper around Apple's unified logging system.

This crate exposes Apple-specific logging macros that require static OSLog
format string literals and emit directly through unified logging. It does not
integrate with the Rust `log` crate; `log` erases the original format string
before a logger backend can preserve OSLog's static format, typed payloads, and
privacy metadata.

## Logging example

```rust
fn main() {
    let log = oslog::OsLog::new("com.example.test", "network");

    let host = "example.com";
    let elapsed_ms = 2.4f64;
    let code = 500i32;
    let token = "secret";

    oslog::debug!(&log, "connecting to %{public}s", host);
    oslog::info!(&log, "connected in %{public}fms", elapsed_ms);
    oslog::error!(&log, "request failed with code %{public}d", code);
    oslog::fault!(&log, "token should stay private: %{private}s", token);
}
```

The format argument must be a string literal. Dynamic format strings are not
accepted by the macros.

The procedural macros validate supported format syntax and argument counts at
compile time. They also generate type-constraining calls for each argument, so a
type mismatch such as this fails during Rust type checking:

```rust,compile_fail
let log = oslog::OsLog::global();
oslog::debug!(&log, "%i", "hi");
```

## Supported format arguments

The macros currently support a focused subset of OSLog/printf conversions:

- signed integers: `%hhd`, `%hd`, `%d`, `%ld`, `%lld`, `%zd`
- unsigned integers: `%hhu`, `%hu`, `%u`, `%lu`, `%llu`, `%zu`
- floats: `%f`, `%e`, `%g`, `%a` and uppercase variants
- characters: `%c`
- strings: `%s`
- pointers: `%p`
- privacy tags: `%{public}...` and `%{private}...`

Unsupported conversions produce a compile error.
