

### 源码构建

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
rustup target add aarch64-linux-android armv7-linux-androideabi aarch64-apple-ios
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