# EZLog

[中文介绍](README.zh-CN.md)</p>
[🦀️Rust 移动端开发体验](./docs/JOURNAL.md)

EZLog is a high efficiency Cross-platform logging library.

it is inspired by [Xlog](https://github.com/Tencent/mars) and [Loagan](https://github.com/Meituan-Dianping/Logan), rewrite in [Rust](https://www.rust-lang.org/).

## Feature
- iOS, Android, MacOS support.
- map file into memory by [mmap](https://man7.org/linux/man-pages/man2/mmap.2.html).
- [zlib](https://en.wikipedia.org/wiki/Zlib) compression.
- [AEAD encryption](https://en.wikipedia.org/wiki/Authenticated_encryption).
- fetch log by callback.
- trim out of date files.
- CLI paser support.

## Quick Start

### iOS

By CocoaPods

#### Add EZLog to Podfile

```shell
pod 'EZLog', '~> 0.1'
```
then

```shell
pod update
```
#### Open Xcode, add sample code

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

3. click run and see console ouput.

### Android

#### Add ezlog to dependencies

open top-level `build.gradle`, add `mavenCentral` to repositories.

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

open app level `build.gradle`, add `ezlog`

```groovy
dependencies {
    implementation "wtf.s1.ezlog:ezlog:0.1.3"
}
```

sync gradle

#### Setup in application. For example

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
<summary><b>Build from source code</b></summary>
</p>
install and config rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

use rust nightly
rustup 1.64.0-nightly-2022-07-15 has a bug, cant compile crate `time`

```shell
rustup default nightly-2022-07-12
```

we use [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) feature, so add nightly src component

```shell
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

add target: iOS, android, etc...

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

clone repository and open in command line tool. then run

```shell
cargo check
```

wait crates download...

```shell
cargo build -p ezlog
```

#### For android build

we use `cargo-ndk` to build dylib

```shell
cargo install cargo-ndk
```

cd android

```shell
sh b_android.sh
```

then open current workspace in AndroidStudio

#### For iOS build

install `cbindgen`

```shell
cargo install --force cbindgen
```

cd ios dir

```shell
sh b_ios.sh
```

open the `ios/EZlog.xcworkspace` in Xcode

</details>

## License

See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), 