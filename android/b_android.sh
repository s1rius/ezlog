#!/bin/bash
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86 -t x86_64 -o lib-ezlog/src/main/jniLibs build -Zbuild-std -p ezlog --features "android_logger" --release