#!/bin/bash

PATH=$PATH:$HOME/.cargo/bin

echo "cargo build for ios"
cargo +nightly build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios --verbose

echo "\n"
echo "cbindgen"
# cbindgen --config cbindgen.toml ../ezlog-core/src/ios.rs > ezlog/ezlog.h