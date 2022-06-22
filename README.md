# ezlog

`ezlog` is a mobile cross-platform file logging library write in rust.

## Feature
- iOS, Android logging support
- Logging compress
- Logging encrypt
- CLI paser support

## Quick Start

### iOS

### Android

<details>
<summary><b>build from source code</b></summary>

install and config rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

use rust nightly default

```shell
rustup default nightly
```

if you already install `rust`, keep update

```shell
rustup update
```

we use [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) feature, so add nightly src component

```shell
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

add target: iOS, android, etc...

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi aarch64-apple-ios
```

clone repository and open in command line tool. then run

```shell
cargo check
```

wait crates download...

```shell
cargo build -p ezlog
```

#### for android build

we use `cargo-ndk` to build dylib

```shell
cargo install cargo-ndk
```

cd android

```shell
sh b_android.sh
```

then open current workspace in AndroidStudio

#### for iOS build

install `cbindgen`

```shell
cargo install --force cbindgen
```

cd ios dir

```shell
sh b_ios.sh
```

open the ios dir in Xcode

</details>
