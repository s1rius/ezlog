//
//  EZSample.swift
//  demo
//
//  Created by s1rius on 2022/11/18.
//

import Foundation
import EZLog
import benchmarkable

func ezlogSampleInit() {
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
    _ = EZLogger(config: config)
    
    
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
}

func aLog() {
    log(logName: "demo", level: Level.trace, tag: "test", msg: randomString(length: 100))
}

func logs() {
    for _ in 0...10000 {
        aLog()
    }
}

func logInOtherThread() {
    DispatchQueue(label: "ezlog queue").async {
        pthread_setname_np("thread2")
        log(logName: "demo", level: Level.trace, tag: "test", msg: String(format: "background log %@", Thread.current.name!))
    }
}

func reqeustLogs(date: String) {
    requestLogsForDate(logName: "demo", date: date)
    ezlog_trim()
}

func requestLogsByDate(date: Date) {
    let date = Date()
    let df = DateFormatter()
    df.dateFormat = "yyyy_MM_dd"
    let dateString = df.string(from: date)
    reqeustLogs(date: dateString)
}