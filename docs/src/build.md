# Build

- install and config rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

- use rust nightly

```shell
rustup default nightly-2022-08-10
```

we use [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) feature, so add nightly src component

```shell
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

clone repository and open in command line tool. then run

```shell
cargo check
```

wait crates download...

```shell
cargo build -p ezlog
```

## for Flutter build

```dart
flutter packages get

flutter packages upgrade
```

## For android build

- add android targets

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

we use `cargo-ndk` to build dylib

```shell
cargo install cargo-ndk
```

cd android

```shell
sh b_android.sh
```

then open current workspace in AndroidStudio

## For iOS build

- add iOS targets

```shell
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```


install `cbindgen`

```shell
cargo install --force cbindgen
```

cd ios dir

```shell
sh b_ios.sh
```

open the `ios/EZlog.xcworkspace` in Xcode