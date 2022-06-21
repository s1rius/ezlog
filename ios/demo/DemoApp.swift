//
//  demoApp.swift
//  demo
//
//  Created by al dmgmgw on 2022/6/8.
//

import SwiftUI
import ezlog

@main
struct DemoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView().onAppear {
                pthread_setname_np("main")
                #if DEBUG
                ezlogInitWithTrace()
                #else
                ezlogInit()
                #endif
                
                ezlogRegisterCallback(success: {name, date, logs in
                    let c = logs[0]
                    print("name:" + name + " date:" + date + " log:" + c);
                }, fail: {name, date, err in
                    print("name:" + name + " date:" + date + " err:" + err);
                })
                
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
                
                logger.debug("first blood")
                DispatchQueue(label: "ezlog queue").async {
                    pthread_setname_np("ezlog-1")
                    logger.debug(String(format: "background log %@", Thread.current.name!))
                    sleep(3)
                    ezlogRequestLogs(logName: "demo", date: "2022_06_18")
                    logger.debug("log fetched")
                }
            }
        }
    }
}

extension URL {
    static var documents: URL {
        return FileManager
            .default
            .urls(for: .documentDirectory, in: .userDomainMask)[0]
    }
}
