//
//  benchMark.swift
//  benchMark
//
//  Created by s1rius on 2022/11/17.
//

import XCTest
import EZLog
import benchmarkable

final class EZLogBenchMark: XCTestCase, LogCase {
   
    override func setUpWithError() throws {
        // Put setup code here. This method is called before the invocation of each test method in the class.
        let dirPath = URL.documents.appendingPathComponent("ezlog").relativePath
        let config = EZLogConfig(level: Level.trace,
                                 dirPath: dirPath,
                                 name: "demo",
                                 keepDays: 7,
                                 maxSize: 150*1024,
                                 compress: CompressKind.NONE,
                                 compressLevel: CompressLevel.DEFAULT,
                                 cipher: Cipher.NONE,
                                 cipherKey: [UInt8]("a secret key!!!!".utf8),
                                 cipherNonce: [UInt8]("unique nonce".utf8))
        _ = EZLogger(config: config)
    }

    override func tearDownWithError() throws {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
        ezlog_flush_all()
    }
    
    func testOneLog() throws {
        log(logName: "demo", level: Level.trace, tag: "test", msg: randomString(length: 100))
    }
    
    func testLogs() throws {
        measure(options: measureOption()) {
            log(logName: "demo", level: Level.trace, tag: "test", msg: randomString(length: 100))
        }
    }
    
    func testRequestLogFiles() throws {
        let date = Date()
        let df = DateFormatter()
        df.dateFormat = "yyyy_MM_dd"
        let dateString = df.string(from: date)
        requestLogsForDate(logName: "demo", date: dateString)
    }

}