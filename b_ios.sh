#!/bin/bash
cargo +nightly build -Z build-std -p ezlog --release --lib --target aarch64-apple-ios --verbose