# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Add
- cli: add key, nonce to cli options. https://github.com/s1rius/ezlog/pull/29
- add create time info to file header. https://github.com/s1rius/ezlog/pull/38

### Change
- **break** android:rename native function name, remove underline. https://github.com/s1rius/ezlog/pull/26
- use varint to describe log's content length https://github.com/s1rius/ezlog/pull/32
- compress first then encrypt https://github.com/s1rius/ezlog/pull/40

## [0.1.7] (2022-11-24)

### Fix
- fix appender rolling fail https://github.com/s1rius/ezlog/pull/22
- fix get error on request logs path multi times https://github.com/s1rius/ezlog/pull/23
- fix global typo by @nickming https://github.com/s1rius/ezlog/pull/21

### Add
- add ci build android and ios rust lib https://github.com/s1rius/ezlog/pull/17
- flutter: support trim function https://github.com/s1rius/ezlog/pull/18
- add mobile benchmark https://github.com/s1rius/ezlog/pull/20

## [0.1.6] (2022-11-1)
- support trim log files which are out of date 

## [0.1.5] (2022-08-25)

- fix android jni method signature error
- support multi callbacks

## [0.1.4] (2022-07-25)

- update android/iOS prebuild library

## [0.1.3] (2022-07-25)

- ffi hook panic when init
- downgrade ios support version to 13.0
- use Result when index is out of bounds
- add features: decode, backtrace, log

[Unreleased]: https://github.com/s1rius/ezlog/compare/0.1.7...HEAD
[0.1.7]: https://github.com/s1rius/ezlog/compare/0.1.6...0.1.7
[0.1.6]: https://github.com/s1rius/ezlog/compare/0.1.5...0.1.6
[0.1.5]: https://github.com/s1rius/ezlog/compare/0.1.4...0.1.5
[0.1.4]: https://github.com/s1rius/ezlog/compare/0.1.3...0.1.4
[0.1.3]: https://github.com/s1rius/ezlog/compare/0.1.2...0.1.3
