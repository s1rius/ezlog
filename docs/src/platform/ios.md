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
