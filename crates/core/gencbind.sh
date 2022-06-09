#!/bin/bash
cbindgen src/ios.rs -l c > ../../ios/ezlog/ezlog.h
# cargo xcode