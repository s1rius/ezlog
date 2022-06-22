## log file format design

没有设计好压缩加密每一个 log 的数据格式，导致解析出错。
重新参考[GIF](https://en.wikipedia.org/wiki/GIF)格式和[PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics)的设计，分隔符 + 数据长度 + 处理后的二进制数据

实现变长 log 的输出

## project file structure

change lib file struct like [Bevy](https://github.com/bevyengine/bevy)

## how to cross platform use

lean how to write ffi code

### for c

- [rust ffi doc](https://doc.rust-lang.org/nomicon/ffi.html)
- [how to call rust functions from c on linux h37](https://dev.to/dandyvica/how-to-call-rust-functions-from-c-on-linux-h37)
- [how does rust ffi pass parameters of type vec u8](https://users.rust-lang.org/t/how-does-rust-ffi-pass-parameters-of-type-vec-u8/55118)
- [How to return byte array from rust to c](https://users.rust-lang.org/t/how-to-return-byte-array-from-rust-function-to-ffi-c/18136)

### for java

- [Rust bindings to the JNI](https://docs.rs/jni/latest/jni/)
- [JNI crate exapmles](https://github.com/jni-rs/jni-rs/blob/master/example/mylib/src/lib.rs)
- [Implementing JNI_OnLoad](https://github.com/jni-rs/jni-rs/issues/257)

no easy way to get rust ffi binding code from java file, I need implement them by myself.

### for android

- [Rust on Android](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
- [cargo ndk](https://github.com/bbqsrc/cargo-ndk)
- [Minimizing Rust Binary Size](https://github.com/johnthagen/min-sized-rust)

- [Rust 中的 bin, lib, rlib, a, so 概念介绍](https://rustcc.cn/article?id=98b96e69-7a5f-4bba-a38e-35bdd7a0a7dd)

### for ios

- [Create your own CocoaPods library](https://medium.com/@jeantimex/create-your-own-cocoapods-library-da589d5cd270)
- [Building and Deploying a Rust library on iOS via Mozilla](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-06-rust-on-ios.html)
- [Rust on iOS and Mac Catalyst](https://nadim.computer/posts/2022-02-11-maccatalyst.html)
- [recipe swift rust callback](https://www.nickwilcox.com/blog/recipe_swift_rust_callback/)
- [OpenSSl on iOS and MacOs](https://github.com/krzyzanowskim/OpenSSL)
- [From Rust To Swift](https://betterprogramming.pub/from-rust-to-swift-df9bde59b7cd)[github demo](https://github.com/tmarkovski/rust-to-swift)
- [distributing universal ios frameworks as xcframeworks using cocoapods](https://anuragajwani.medium.com/distributing-universal-ios-frameworks-as-xcframeworks-using-cocoapods-699c70a5c961)

### build issue

- build release fail

release flag make build fail, build debug first, then enable release flag.

- build std fail

no function or associated item named `set_name` found for struct `sys::unix::thread::Thread` in the current scope
could not compile `std`


### vscode

Press SHIFT + ALT + I to insert multiple cursors at the end of each line
Press Home twice to jump to the start of every line