//
//  Util.swift
//  benchmark
//
//  Created by s1rius on 2022/11/18.
//

import Foundation

extension URL {
    public static var documents: URL {
        return FileManager
            .default
            .urls(for: .documentDirectory, in: .userDomainMask)[0]
    }
}

public func randomString(length: Int) -> String {
  let letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789,.:;!@#$%^&*()_+-"
  return String((0..<length).map{ _ in letters.randomElement()! })
}
