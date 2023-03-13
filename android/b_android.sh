#!/bin/bash
cargo ndk -t armeabi-v7a -t arm64-v8a -o lib-ezlog/src/main/jniLibs build -Zbuild-std -p ezlog --release
./gradlew :lib-ezlog:assembleRelease