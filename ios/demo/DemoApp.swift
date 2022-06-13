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
                ezlogInit()
                let dirPath = URL.documents.appendingPathComponent("ezlog").relativePath
                let config = EZLogConfig(level: Level.trace,
                                         dirPath: dirPath,
                                         name: "demo",
                                         keepDays: 7,
                                         maxSize: 150*1024,
                                         compress: CompressKind.NONE,
                                         compressLevel: CompressLevel.DEFAULT,
                                         encrypt: EncryptKind.NONE,
                                         encryptKey: [UInt8]("a secret key!!!!".utf8),
                                         encryptNonce: [UInt8]("unique nonce".utf8))
                let logger = EZLogger(config: config)
                
                logger.debug("first blood")
                DispatchQueue(label: "ezlog queue").async {
                    pthread_setname_np("ezlog-1")
                    logger.debug(String(format: "background log %@", Thread.current.name!))
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
