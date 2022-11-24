# CHANGELOG

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## 0.1.7 (2022-11-24)

### Fix
- fix appender rolling fail https://github.com/s1rius/ezlog/pull/22
- fix get error on request logs path multi times https://github.com/s1rius/ezlog/pull/23
- fix global typo by @nickming https://github.com/s1rius/ezlog/pull/21

### Add
- add ci build android and ios rust lib https://github.com/s1rius/ezlog/pull/17
- flutter: support trim function https://github.com/s1rius/ezlog/pull/18
- add mobile benchmark https://github.com/s1rius/ezlog/pull/20

## 0.1.6 (2022-11-1)
- support trim log files which are out of date 

## 0.1.5 (2022-08-25)

- fix android jni method signature error
- support multi callbacks

## 0.1.4 (2022-07-25)

- update android/iOS prebuild library

## 0.1.3 (2022-07-25)

- ffi hook panic when init
- downgrade ios support version to 13.0
- use Result when index is out of bounds
- add features: decode, backtrace, log

