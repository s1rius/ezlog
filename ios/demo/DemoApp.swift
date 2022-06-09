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
                print("hello world")
                ezlogInit()
                let dirPath = URL.documents.appendingPathComponent("ezlog").relativePath
                print(FileManager.default.fileExists(atPath: dirPath))
                let config = EZLogConfig(level: Level.trace,
                                         dirPath: dirPath,
                                         name: "demo",
                                         keepDays: 7,
                                         maxSize: 150*1024,
                                         compress: CompressKind.ZLIB,
                                         compressLevel: CompressLevel.DEFAULT,
                                         encrypt: EncryptKind.AES128GCM,
                                         encryptKey: [UInt8]("a secret key!!!!".utf8),
                                         encryptNonce: [UInt8]("unique nonce".utf8))
                let logger = EZLogger(config: config)
                
                logger.debug("first blood")
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
