//
//  LoganBenchmark.swift
//  benchmark
//
//  Created by s1rius on 2022/11/18.
//

import Foundation
import XCTest

import Logan
import benchmarkable

final class LoganBenchMark: XCTestCase, LogCase {
    
    override func setUpWithError() throws {
        let keydata = "0123456789012345".data(using: .utf8)!
        let ivdata = "0123456789012345".data(using: .utf8)!
        let file_max: UInt64 = 10 * 1024 * 1024;
        
        
        // logan init，incoming 16-bit key，16-bit iv，largest written to the file size(byte)
        loganInit(keydata, ivdata, file_max);
    }

    override func tearDownWithError() throws {
        // Put teardown code here. This method is called after the invocation of each test method in the class.
        loganFlush()
    }
    
    func testOneLog() throws {
        logan(1, randomString(length: 100))
    }
    
    func testLogs() throws {
        measure(options: measureOption()) {
            loganlog()
        }
    }
    
    func testRequestLogFiles() throws {
        
    }
    
    func loganlog() {
        logan(1, randomString(length: 100))
    }
    

}
