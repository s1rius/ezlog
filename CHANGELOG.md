# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Add
- cli: Add key, nonce to cli options https://github.com/s1rius/ezlog/pull/29
- Add create time info to file header https://github.com/s1rius/ezlog/pull/38
- Make file rorate duration configurable https://github.com/s1rius/ezlog/pull/46
- Add log file header extra info https://github.com/s1rius/ezlog/pull/48
- Add AES-GCM-SIV encryption as mandatory for v2 https://github.com/s1rius/ezlog/pull/56
- Generate a unique nonce for each encryption instance https://github.com/s1rius/ezlog/pull/56
- Make log record format configurable https://github.com/s1rius/ezlog/pull/66
- Add integration test https://github.com/s1rius/ezlog/pull/67
- Rotate file when the query date is today https://github.com/s1rius/ezlog/pull/70

### Change
- **break** android:rename native function name, remove underline https://github.com/s1rius/ezlog/pull/26
- Use varint to describe log's content length https://github.com/s1rius/ezlog/pull/32
- Compress first then encrypt https://github.com/s1rius/ezlog/pull/40

### Fix
- Auto rotate log file, when the config is not match previous https://github.com/s1rius/ezlog/pull/60

## [0.1.7] (2022-11-24)

### Fix
- Fix appender rolling fail https://github.com/s1rius/ezlog/pull/22
- Fix get error on request logs path multi times https://github.com/s1rius/ezlog/pull/23
- Fix global typo by @nickming https://github.com/s1rius/ezlog/pull/21

### Add
- Add ci build android and ios rust lib https://github.com/s1rius/ezlog/pull/17
- flutter: support trim function https://github.com/s1rius/ezlog/pull/18
- Add mobile benchmark https://github.com/s1rius/ezlog/pull/20

## [0.1.6] (2022-11-1)
- Support trim log files which are out of date 

## [0.1.5] (2022-08-25)

- Fix android jni method signature error
- Support multi callbacks

## [0.1.4] (2022-07-25)

- Update android/iOS prebuild library

## [0.1.3] (2022-07-25)

- FFI hook panic when init
- Downgrade ios support version to 13.0
- Use Result when index is out of bounds
- Add features: decode, backtrace, log

[Unreleased]: https://github.com/s1rius/ezlog/compare/0.1.7...HEAD
[0.1.7]: https://github.com/s1rius/ezlog/compare/0.1.6...0.1.7
[0.1.6]: https://github.com/s1rius/ezlog/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/s1rius/ezlog/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/s1rius/ezlog/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/s1rius/ezlog/compare/0.1.2...0.1.3
