# ezlog

[中文介绍](https://s1rius.github.io/ezlog/zh/index.html)</p>

## What is ezlog?

ezlog is a high-performance cross-platform file logging library.

It can be used in Flutter, Android, iOS, Windows, Linux, MacOS.

It is inspired by [Xlog](https://github.com/Tencent/mars) and [Logan](https://github.com/Meituan-Dianping/Logan), rewrite in Rust.

### Features
- multi platform: Flutter, Android, iOS, Windows, Linux, MacOS
- map file into memory by [mmap](https://man7.org/linux/man-pages/man2/mmap.2.html).
- compression support, eg: [zlib](https://en.wikipedia.org/wiki/Zlib).
- encryption support, eg: [AEAD encryption](https://en.wikipedia.org/wiki/Authenticated_encryption).
- fetch log by callback.
- trim out of date files.
- command line parser support.

### License

See [LICENSE-MIT](../../LICENSE-MIT), [LICENSE-APACHE](../../LICENSE-APACHE)
### Android Usage

#### Add ezlog to dependencies

Open top-level `build.gradle`, add `mavenCentral` to repositories.

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

Open app level `build.gradle`, add `ezlog`

```groovy
dependencies {
    implementation "wtf.s1.ezlog:ezlog:0.2+"
}
```

Sync gradle

#### Setup in application

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

    EZLog.registerCallback(object : Callback {
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
### Flutter Usage

#### Add ezlog_flutter as a dependency in your pubspec.yaml file.

```yaml
dependencies:
  ezlog_flutter: ^0.2.0
```

#### Example

```dart
import 'dart:io';
import 'package:flutter/material.dart';
import 'dart:async';
import 'package:ezlog_flutter/ezlog_flutter.dart';
import 'package:path_provider/path_provider.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {

  @override
  void initState() {
    super.initState();
    initEZLog();
  }

  Future<void> initEZLog() async {
    EZLog.init(true);
    Directory appDocDir = await getApplicationSupportDirectory();
    String logDir = '${appDocDir.path}/ezlog';

    var logger = EZLogger.config(
        EZLogConfig.plaintext("main", Level.trace.id, logDir, 7));
    
    logger.d("init", "success");

    var logs = await EZLog.requestLogFilesForDate("main", "2022_08_25");
  }
}
```
### iOS Usage

#### Add ezlog

Add dependency to Podfile

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
click run and see console ouput.
### Rust Usage

#### Add ezlog

Add this to your Cargo.toml

```toml
[dependencies]
ezlog = "0.2"
```

#### Example

```rust
use ezlog::EZLogConfigBuilder;
use ezlog::Level;
use log::{error, info, warn};
use log::{LevelFilter, Log};

ezlog::InitBuilder::new().init();

let config = EZLogConfigBuilder::new()
        .level(Level::Trace)
        .dir_path(
            dirs::download_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .expect("dir path error"),
        )
        .build();
ezlog::create_log(config);

info!("hello ezlog");

```

see more examples in examples dir.
## Architecture

### Code structure

```
├── android
│   ├── app # android demo app
│   └── lib-ezlog # ezlog android library
├── examples # Rust examples
├── ezlog_flutter # Flutter plugin
├── ezlogcli # Rust command line tool
├── ezlog-core # Rust core library
├── ios
│   ├── EZLog # ezlog iOS library
│   ├── demo # iOS demo app
│   └── framework # ezlog XCFramework
```

### Log file format

#### Header 

| Bytes Offset | Meaning                            |
|--------|------------------------------------------|
| 0-1    | 'ez'                                     |
| 2      | Version number                           |
| 3      | Flag bits                                |
| 4-7    | Offset of recorder position in bytes     |
| 8-15   | Unix timestamp (big-endian)              |
| 16     | Compression type                         |
| 17     | Encryption type                          |
| 18-21  | Encryption key hash                      |

#### Per log record

| Byte Offset | Field Name| Description  |
|----------|-----------|-----------------|
| 0| Start Byte| Always 0x3b indicating the start|
| 1-varint|Record Length| A variable-length integer that specifies the length|
| varint+1-varint+n | Record Content | The actual log record content |
| varint+n+1| End Byte| Always 0x21 indicating the start |

### Compression

We use zlib as the compression algorithm.

### Encryption

#### We use AES-GCM-SIV as the encryption algorithm.

AES-GCM-SIV, as a symmetric encryption algorithm, is more efficient compared to asymmetric encryption. As an AEAD, When compared to AES-CFB, it is more secure, and when compared to AES-GCM, AES-GCM-SIV is nonce-misuse-resistant.

### Make nonce not repeat

First of all, we need an init nonce, which is generated randomly when the logger is created. Then, we get the timestamp of the log file creation. When we write a log record, we know the current index of the log file, and we can calculate the nonce of the current log record by the following formula:

```
nonce = init_nonce ^ timestamp.extend(index)

```
## Benchmark

### Android Benchmark

#### measure log method

| Library | Time (ns) | Allocations |
|---------|-----------|-------------|
| logcat  | 2,427     | 7           |
| logan   | 4,726     | 14          |
| ezlog   | 8,404     | 7           |
| xlog    | 12,459    | 7           |

#### startup time

startup baseline
```
min 206.4,   median 218.5,   max 251.9
```

startup with ezlog time:
```
min 206.8,   median 216.6,   max 276.6
```
## Build

- install and config rust

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

- use rust nightly

```shell
rustup default nightly-2022-08-10
```

we use [build-std](https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#build-std) feature, so add nightly src component

```shell
rustup component add rust-src --toolchain nightly-x86_64-apple-darwin
```

clone repository and open in command line tool. then run

```shell
cargo check
```

wait crates download...

```shell
cargo build -p ezlog
```

### for Flutter build

```dart
flutter packages get

flutter packages upgrade
```

### For android build

- add android targets

```shell
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

we use `cargo-ndk` to build dylib

```shell
cargo install cargo-ndk
```

cd android

```shell
sh b_android.sh
```

then open current workspace in AndroidStudio

### For iOS build

- add iOS targets

```shell
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```


install `cbindgen`

```shell
cargo install --force cbindgen
```

cd ios dir

```shell
sh b_ios.sh
```

open the `ios/EZlog.xcworkspace` in Xcode
