# EZLog是一个高效的跨平台的日志库
EZLog灵感来自([Xlog](https://github.com/Tencent/mars)和[Loagan](https://github.com/Meituan-Dianping/Logan)，用[Rust](https://www.rust-lang.org/)重写。

## 特性
- iOS, Android, MacOS 支持
- 使用[mmap](https://man7.org/linux/man-pages/man2/mmap.2.html)做日志映射
- 认证加密[Authenticated encryption - Wikipedia](https://en.wikipedia.org/wiki/Authenticated_encryption)
- ZLIB压缩[zlib](https://en.wikipedia.org/wiki/Zlib)
- 日志回捞
- 日志清理
- 命令行解析工具

## 快速开始
### iOS

### Android



<details>
<summary><b>从源码构建</b></summary>

安装配置`rust`

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

使用nightly版本

``` 
rustup default nightly 
```

保证`rust`是最新版

```
rustup update
```

构建时使用了[build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std)特性，所以需要添加std的源码组件

```
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

添加构建目标: iOS, android

```
rustup target add aarch64-linux-android armv7-linux-androideabi aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

克隆仓库到本地后，在命令行中执行

```
cargo check
```

等待所有的依赖下载...构建ezlog包

```
cargo build -p ezlog
```

####  android工程构建

使用`cargo-ndk`来构建动态链接库

```
cargo install cargo-ndk
```

在仓库的android目录下执行

```
sh b_android.sh
```

在AndroidStudio中打开android项目


#### iOS工程构建

安装 `cbindgen`

```
cargo install --force cbindgen
```

在仓库的ios目录执行

```
sh b_ios.sh
```

在Xcode里打开ios项目

</details>