#!/bin/bash

PATH=$PATH:$HOME/.cargo/bin

echo "cargo build for iOS"
cargo build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios --verbose
cargo build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios-sim --verbose
cargo build -Z build-std -p ezlog --release --lib --target x86_64-apple-ios --verbose

echo "\n"
# echo "cbindgen"
# cbindgen --config cbindgen.toml ../ezlog-core/src/ffi_c.rs > ezlog/Sources/ezlog.h

mkdir -p ../target/fat-ios-sim/release
rm -rf ../target/fat-ios-sim/release/libezlog.a

lipo -create ../target/aarch64-apple-ios-sim/release/libezlog.a ../target/x86_64-apple-ios/release/libezlog.a -output ../target/fat-ios-sim/release/libezlog.a

rm -rf framework/ezlog.xcframework

xcodebuild -create-xcframework \
    -library ../target/aarch64-apple-ios/release/libezlog.a \
    -headers EZLog/Sources/ezlog.h \
    -library ../target/fat-ios-sim/release/libezlog.a \
    -headers EZLog/Sources/ezlog.h \
    -output framework/ezlog.xcframework