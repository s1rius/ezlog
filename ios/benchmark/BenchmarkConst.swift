//
//  BenchConst.swift
//  benchmark
//
//  Created by s1rius on 2022/11/20.
//

import Foundation
import XCTest


func measureOption() -> XCTMeasureOptions {
    let options = XCTMeasureOptions()
    options.iterationCount = 10000
    return options
}
