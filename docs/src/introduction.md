# ezlog

## What is ezlog?

ezlog is a high-performance cross-platform file logging library.

It can be used in Flutter, Android, iOS, Windows, Linux, MacOS.

It is inspired by [Xlog](https://github.com/Tencent/mars) and [Logan](https://github.com/Meituan-Dianping/Logan), rewrite in Rust.

## Features
- multi platform: Flutter, Android, iOS, Windows, Linux, MacOS
- map file into memory by [mmap](https://man7.org/linux/man-pages/man2/mmap.2.html).
- compression support, eg: [zlib](https://en.wikipedia.org/wiki/Zlib).
- encryption support, eg: [AEAD encryption](https://en.wikipedia.org/wiki/Authenticated_encryption).
- fetch log by callback.
- trim out of date files.
- command line parser support.

## License

See [LICENSE-MIT](../../LICENSE-MIT), [LICENSE-APACHE](../../LICENSE-APACHE), 