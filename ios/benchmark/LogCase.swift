//
//  LogCase.swift
//  benchmark
//
//  Created by s1rius on 2022/11/20.
//

import Foundation

protocol LogCase {
    func testOneLog() throws
    func testLogs() throws
    func testRequestLogFiles() throws
}
