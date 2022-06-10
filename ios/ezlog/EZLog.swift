//
//  EZLog.swift
//  ezlog
//
//  Created by al dmgmgw on 2022/6/8.
//

import Foundation

public struct EZLogger {
    var config: EZLogConfig
    
    public init(config: EZLogConfig) {
        self.config = config
        create(config: config)
    }
    
    func log(level: Level, message: String, target: String? = "") {
        ezlog_log(self.config.name, UInt8(level.rawValue), target, message)
    }
    
    func flush() {
        ezlog_flush(self.config.name)
    }
}

extension EZLogger {

    public func trace(_ message: @autoclosure () -> String, target: @autoclosure() -> String? = "") {
        self.log(level: Level.trace, message: message(), target: target())
    }
    
    public func debug(_ message: @autoclosure () -> String, target: @autoclosure() -> String? = "") {
        self.log(level: Level.debug, message: message(), target: target())
    }
    
    public func info(_ message: @autoclosure () -> String, target: @autoclosure() -> String? = "") {
        self.log(level: Level.info, message: message(), target: target())
    }
    
    public func warn(_ message: @autoclosure () -> String, target: @autoclosure() -> String? = "") {
        self.log(level: Level.warning, message: message(), target: target())
    }
    
    public func error(_ message: @autoclosure () -> String, target: @autoclosure() -> String? = "") {
        self.log(level: Level.error, message: message(), target: target())
    }
}

private func create(config: EZLogConfig) {
    ezlog_create_log(config.name,
                 UInt8(config.level.rawValue),
                 config.dirPath,
                 UInt32(config.keepDays),
                 UInt8(config.compress.rawValue),
                 UInt8(config.compressLevel.rawValue),
                 UInt8(config.encrypt.rawValue),
                 config.encryptKey,
                 UInt(config.encryptKey.count),
                 config.encryptNonce,
                 UInt(config.encryptNonce.count))
}

public func ezlogInit() {
    ezlog_init()
}

public func flushAll() {
    ezlog_flush_all()
}

/// The log level.
///
/// Log levels are ordered by their severity, with `.trace` being the least severe and
/// `.error` being the most severe.
public enum Level: Int, Codable {
    /// Appropriate for error conditions.
    case error = 1
    
    /// Appropriate for messages that are not error conditions, but more severe than
    /// `.notice`.
    case warning
    
    /// Appropriate for informational messages.
    case info
    
    /// Appropriate for messages that contain information normally of use only when
    /// debugging a program.
    case debug
    
    /// Appropriate for messages that contain information normally of use only when
    /// tracing the execution of a program.
    case trace
}

public enum CompressKind: Int, Codable {
    case NONE = 0
    case ZLIB = 1
}

public enum EncryptKind: Int {
    case NONE = 0
    case AES128GCM
    case AES256GCM
}

public enum CompressLevel: Int, Codable {
    case DEFAULT = 0
    case FAST = 1
    case BEST = 2
}

public struct EZLogConfig {
    var level: Level
    var dirPath: String
    var name: String
    var keepDays: Int
    var maxSize: Int
    var compress: CompressKind
    var compressLevel: CompressLevel
    var encrypt: EncryptKind = EncryptKind.NONE
    var encryptKey: [UInt8]
    var encryptNonce: [UInt8]
    
    public init(
        level: Level,
        dirPath: String,
        name: String,
        keepDays: Int,
        maxSize: Int,
        compress: CompressKind,
        compressLevel: CompressLevel,
        encrypt: EncryptKind,
        encryptKey: [UInt8],
        encryptNonce: [UInt8]
    ) {
        self.level = level
        self.dirPath = dirPath
        self.name = name
        self.keepDays = keepDays
        self.maxSize = maxSize
        self.compress = compress
        self.compressLevel = compressLevel
        self.encrypt = encrypt
        self.encryptKey = encryptKey
        self.encryptNonce = encryptNonce
    }
}
