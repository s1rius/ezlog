# EZLog是一个高效的跨平台的日志库
EZLog灵感来自[Xlog](https://github.com/Tencent/mars)和[Loagan](https://github.com/Meituan-Dianping/Logan)，用[Rust](https://www.rust-lang.org/)重写。

## 特性
- iOS, Android, MacOS 支持
- 使用[mmap](https://man7.org/linux/man-pages/man2/mmap.2.html)做日志映射
- [认证加密](https://en.wikipedia.org/wiki/Authenticated_encryption)
- [zlib](https://en.wikipedia.org/wiki/Zlib)压缩
- 日志回捞
- 日志清理
- 命令行解析工具

## 快速开始
### iOS

使用CocoaPods管理依赖

#### 添加 EZLog 到 Podfile

```
pod 'EZLog', '~> 0.1'
```
更新

```
pod update
```
#### 打开Xcode, 添加示例代码

```swift
import EZLog

init() {
    pthread_setname_np("main")
    #if DEBUG
    ezlogInitWithTrace()
    #else
    ezlogInit()
    #endif
    
    let dirPath = URL.documents.appendingPathComponent("ezlog").relativePath
    let config = EZLogConfig(level: Level.trace,
                                dirPath: dirPath,
                                name: "demo",
                                keepDays: 7,
                                maxSize: 150*1024,
                                compress: CompressKind.ZLIB,
                                compressLevel: CompressLevel.DEFAULT,
                                cipher: Cipher.AES128GCM,
                                cipherKey: [UInt8]("a secret key!!!!".utf8),
                                cipherNonce: [UInt8]("unique nonce".utf8))
    let logger = EZLogger(config: config)

    ezlogRegisterCallback(success: {name, date, logs in
        if !logs.isEmpty {
            for log in logs {
                print("name:" + name + " date:" + date + " log:" + log);
            }
        } else {
            print("no log found at that time")
        }
        
    }, fail: {name, date, err in
        print("name:" + name + " date:" + date + " err:" + err);
    })
    
    logger.debug("first blood")
}
```

3. 点击运行，查看控制台输出

### Android

#### 添加依赖

打开项目顶层`build.gradle`文件, 添加`mavenCentral`

```groovy
buildscript {
    repositories {
        ...
        mavenCentral()
        ...
    }
}

allprojects {
    repositories {
        ...
        mavenCentral()
        ...
    }
}
```

在应用目录的`build.gradle`文件中, 添加`ezlog`依赖

```
dependencies {
    implementation "wtf.s1.ezlog:ezlog:0.1.4"
}
```

同步 gradle

#### 在应用中初始化。例如

```kotlin
override fun onCreate() {
    super.onCreate()

    val path = File(filesDir, "ezlog").absolutePath
    val config = EZLogConfig.Builder("demo", path)
        .compress(EZLog.CompressZlib)
        .compressLevel(EZLog.CompressFast)
        .cipher(EZLog.Aes128Gcm)
        .cipherKey("a secret key!!!!".toByteArray())
        .cipherNonce("unique nonce".toByteArray())
        .enableTrace(BuildConfig.DEBUG)
        .build()
    EZLog.initWith(config)

    EZLog.v("ezlog", "first blood")

    EZLog._registerCallback(object : Callback {
        override fun onLogsFetchSuccess(
            logName: String?,
            date: String?,
            logs: Array<out String>?
        ) {
            Log.i("ezlog", "$logName $date ${logs.contentToString()}")
            logs?.let {
                logs.getOrNull(0)?.let { log ->
                    Log.i("ezlog", "check file exists ${File(log).exists()}")
                }
            }
        }

        override fun onLogsFetchFail(logName: String?, date: String?, err: String?) {
            Log.i("ezlog", "$logName $date $err")
        }
    })
}

```

<details>
<summary><b>从源码构建</b></summary>
</p>

安装配置`Rust`

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

使用nightly版本

``` 
rustup default nightly-2022-07-12
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

在Xcode里打开`ios/EZlog.xcworkspace`

</details>

## 协议

详见 [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), 