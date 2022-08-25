//
//  demoApp.swift
//  demo
//
//  Created by al dmgmgw on 2022/6/8.
//

import SwiftUI
import EZLog

@main
struct DemoApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
    
    public init() {
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
        
        
        addCallback(callback: EZCallback( success: {name, date, logs in
            if !logs.isEmpty {
                for log in logs {
                    print("name:" + name + " date:" + date + " log:" + log);
                }
            } else {
                print("no log found at that time")
            }
            
        }, fail: {name, date, err in
            print("name:" + name + " date:" + date + " err:" + err);
        }))
        
        logger.debug("first blood")
        
        DispatchQueue(label: "ezlog queue").async {
            pthread_setname_np("ezlog-callback")
            logger.debug(String(format: "background log %@", Thread.current.name!))
            sleep(3)
            requestLogsForDate(logName: "demo", date: "2022_06_18")
            logger.debug("log fetched")
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
