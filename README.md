# ezlog

ezlog is 

### build from source code

install and config rust

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

default nightly

``` 
rustup default nightly 
```

if you already install `rust`, keep update

```
rustup update
```

we use [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) feature, so add nightly src component

```
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

add target: iOS, android, etc...

```
rustup target add aarch64-linux-android armv7-linux-androideabi aarch64-apple-ios
```

clone repository and open in command line tool. then run

```
cargo check
```

wait crates download...

```
cargo build -p ezlog
```

#### for android build

we use `cargo-ndk` to build dylib

```
cargo install cargo-ndk
```

cd android

```
sh b_android.sh
```

then open current workspace in AndroidStudio


#### for iOS build

install `cbindgen`

```
cargo install --force cbindgen
```

cd ios dir

```
sh b_ios.sh
```

open the ios dir in Xcode