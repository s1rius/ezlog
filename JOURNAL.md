
## log file format design

没有设计好压缩加密每一个log的数据格式，导致解析出错。
重新参考[GIF](https://en.wikipedia.org/wiki/GIF)格式和[PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics)的设计，分隔符 + 数据长度 + 处理后的二进制数据

实现变长log的输出


## project file structure

change lib file struct like [Bevy](https://github.com/bevyengine/bevy)

## how to cross platform use

lean how to write ffi code

### for c

- [rust ffi doc](https://doc.rust-lang.org/nomicon/ffi.html)
- [how to call rust functions from c on linux h37](https://dev.to/dandyvica/how-to-call-rust-functions-from-c-on-linux-h37)
- [how does rust ffi pass parameters of type vec u8](https://users.rust-lang.org/t/how-does-rust-ffi-pass-parameters-of-type-vec-u8/55118)

### for java

- [Rust bindings to the JNI](https://docs.rs/jni/latest/jni/)
- [JNI crate exapmles](https://github.com/jni-rs/jni-rs/blob/master/example/mylib/src/lib.rs)
- [Rust and the JVM](https://blog.frankel.ch/start-rust/7/)

no easy way to get rust ffi binding code from java file, I need implement them by myself.

### for android
- [Rust on Android](https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-21-rust-on-android.html)
- [cargo ndk](https://github.com/bbqsrc/cargo-ndk)
- [Minimizing Rust Binary Size](https://github.com/johnthagen/min-sized-rust)
