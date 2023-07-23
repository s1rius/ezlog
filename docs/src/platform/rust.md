# Rust ezlog

### Usage

Add this to your Cargo.toml

```toml
[dependencies]
ezlog = "0.2"
```


### Example

```rust
use ezlog::EZLogConfigBuilder;
use ezlog::Level;
use log::{error, info, warn};
use log::{LevelFilter, Log};

ezlog::InitBuilder::new().init();

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

```

see more examples in examples dir.