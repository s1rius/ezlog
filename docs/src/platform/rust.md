### Rust Usage

#### Add ezlog

Add this to your Cargo.toml

```toml
[dependencies]
ezlog = "0.2"
```

#### Example

```rust
use ezlog::EZLogConfigBuilder;
use ezlog::Level;
use log::{error, info, warn};
use log::{LevelFilter, Log};

let on_success: fn(&str, &str, &[&str]) = |name, date, logs| {
    println!(
        "on_success: name: {}, desc: {}, tags: {:?}", name, date, logs
    );
};

let on_fail: fn(&str, &str, &str) = |name, date, err| {
    println!(
        "on_fail: name: {}, desc: {}, err: {}", name, date, err
    );
};

ezlog::InitBuilder::new()
    .with_request_callback_fn(on_success, on_fail)
    .init();

let config = EZLogConfigBuilder::new()
        .level(Level::Trace)
        .dir_path(
            dirs::download_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .expect("dir path error"),
        )
        .build();
ezlog::create_log(config);

info!("hello ezlog");

ezlog::request_log_files_for_date(
    ezlog::DEFAULT_LOG_NAME,
    OffsetDateTime::now_utc(),
    OffsetDateTime::now_utc()
);

```

see more examples in examples dir.
