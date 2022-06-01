## 05-26

没有设计好压缩加密每一个log的数据格式，导致解析出错。
重新参考[GIF](https://en.wikipedia.org/wiki/GIF)格式和[PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics)的设计，分隔符 + 数据长度 + 处理后的二进制数据

## 05-28 
实现变长log的输出


## 06-01

change lib file struct like [Bevy](https://github.com/bevyengine/bevy)

lean how to write ffi code

- [rust ffi doc](https://doc.rust-lang.org/nomicon/ffi.html)
- [how to call rust functions from c on linux h37](https://dev.to/dandyvica/how-to-call-rust-functions-from-c-on-linux-h37)
- [how does rust ffi pass parameters of type vec u8](https://users.rust-lang.org/t/how-does-rust-ffi-pass-parameters-of-type-vec-u8/55118)

find rust to jni crate

[Rust bindings to the JNI](https://docs.rs/jni/latest/jni/)
[Rust and the JVM](https://blog.frankel.ch/start-rust/7/)

no easy way to get rust ffi binding code from java file, I need implement them by myself.



