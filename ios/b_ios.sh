#!/bin/bash

PATH=$PATH:$HOME/.cargo/bin

echo "cargo build for iOS"
cargo +nightly build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios --verbose
cargo +nightly build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios-sim --verbose
cargo +nightly build -Z build-std -p ezlog --release --lib --target x86_64-apple-ios --verbose

echo "\n"
# echo "cbindgen"
# cbindgen --config cbindgen.toml ../ezlog-core/src/ios.rs > ezlog/ezlog.h

mkdir -p ../target/fat-ios-sim/release
rm -rf ../target/fat-ios-sim/release/libezlog.a

lipo -create ../target/aarch64-apple-ios-sim/release/libezlog.a ../target/x86_64-apple-ios/release/libezlog.a -output ../target/fat-ios-sim/release/libezlog.a

rm -rf framework/EZLogFramework.xcframework

xcodebuild -create-xcframework \
    -library ../target/aarch64-apple-ios/release/libezlog.a \
    -headers ezlog/Sources/ezlog.h \
    -library ../target/fat-ios-sim/release/libezlog.a \
    -headers ezlog/Sources/ezlog.h \
    -output framework/EZLogFramework.xcframework