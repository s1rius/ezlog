//
//  EZLog.swift
//  ezlog
//
//  Created by s1rius on 2022/6/8.
//

import Foundation

public struct EZLogger {
    var config: EZLogConfig
    
    public init(config: EZLogConfig) {
        self.config = config
        ezlogCreate(config: config)
    }
    
    func log(level: Level, message: String, target: String = "") {
        ezlog_log(self.config.name, UInt8(level.rawValue), target, message)
    }
    
    func flush() {
        ezlog_flush(self.config.name)
    }
}

extension EZLogger {
    
    public func trace(_ message: @autoclosure () -> String, target: @autoclosure() -> String = "") {
        self.log(level: Level.trace, message: message(), target: target())
    }
    
    public func debug(_ message: @autoclosure () -> String, target: @autoclosure() -> String = "") {
        self.log(level: Level.debug, message: message(), target: target())
    }
    
    public func info(_ message: @autoclosure () -> String, target: @autoclosure() -> String = "") {
        self.log(level: Level.info, message: message(), target: target())
    }
    
    public func warn(_ message: @autoclosure () -> String, target: @autoclosure() -> String = "") {
        self.log(level: Level.warning, message: message(), target: target())
    }
    
    public func error(_ message: @autoclosure () -> String, target: @autoclosure() -> String = "") {
        self.log(level: Level.error, message: message(), target: target())
    }
}

private func ezlogCreate(config: EZLogConfig) {
    ezlog_create_log(config.name,
                     UInt8(config.level.rawValue),
                     config.dirPath,
                     UInt32(config.keepDays),
                     UInt8(config.compress?.rawValue ?? 0),
                     UInt8(config.compressLevel?.rawValue ?? 0),
                     UInt8(config.cipher?.rawValue ?? 0),
                     config.cipherKey ?? [],
                     UInt(config.cipherKey?.count ?? 0),
                     config.cipherNonce ?? [],
                     UInt(config.cipherNonce?.count ?? 0),
                     UInt32(config.rotateHours ?? 24),
                     config.extra ?? ""
    )
}

public func ezlogInit() {
    ezlog_init(false)
}

public func ezlogInitWithTrace() {
    ezlog_init(true)
}

public func flushAll() {
    ezlog_flush_all()
}

public func log(logName: String, level: Level, tag: String, msg: String) {
    ezlog_log(logName, UInt8(level.rawValue), tag, msg)
}

public func flush(logName: String) {
    ezlog_flush(logName)
}

public func trim() {
    ezlog_trim()
}

public func requestLogsForDate(logName: String, date: String) {
    ezlog_request_log_files_for_date(logName, date)
}

private class WrapClosure<T> {
    fileprivate let closure: T
    init(closure: T) {
        self.closure = closure
    }
}

public func wrapCallback(success: @escaping (String, String, [String]) -> Void,
                         fail: @escaping (String, String, String) -> Void) -> Callback {
    let successWrapper = WrapClosure(closure: success)
    let successPoint =  Unmanaged.passRetained(successWrapper).toOpaque()
    let success: @convention(c)(UnsafeMutableRawPointer,
                                UnsafePointer<CChar>,
                                UnsafePointer<CChar>,
                                UnsafePointer<UnsafePointer<Int8>>,
                                Int32) -> Void = {
        (s: UnsafeMutableRawPointer,
         logName: UnsafePointer<CChar>,
         date: UnsafePointer<CChar>,
         logs: UnsafePointer<UnsafePointer<Int8>>,
         size: Int32) in
        
        let successWrapper:WrapClosure<(String,String,[String]) -> Void> = Unmanaged.fromOpaque(s).takeUnretainedValue()
        
        var strings: [String] = []
        let bufPoint = Array(UnsafeBufferPointer(start: logs, count: Int(size)))
        for perStrPoint in bufPoint {
            strings.append(String(cString: perStrPoint))
        }
        
        successWrapper.closure(String(cString: logName), String(cString: date), strings)
    }
    
    let failWrapper =  WrapClosure(closure: fail)
    let failPoint = Unmanaged.passRetained(failWrapper).toOpaque()
    let fail: @convention(c)(UnsafeMutableRawPointer,
                             UnsafePointer<CChar>,
                             UnsafePointer<CChar>,
                             UnsafePointer<CChar>) -> Void = {
        (f: UnsafeMutableRawPointer,
         logName: UnsafePointer<CChar>,
         date: UnsafePointer<CChar>,
         err: UnsafePointer<CChar>) in
        
        let failWrapper: WrapClosure<(String, String, String) -> Void> = Unmanaged.fromOpaque(f).takeUnretainedValue()
        failWrapper.closure(String(cString: logName), String(cString: date), String(cString: err))
    }
    return Callback(successPoint: successPoint, onLogsFetchSuccess: success, failPoint: failPoint, onLogsFetchFail: fail)
}

public func ezlogRegisterCallback(success: @escaping (String, String, [String]) -> Void,
                                  fail: @escaping (String, String, String) -> Void) {
    addCallback(callback: EZCallback(success: success, fail: fail))
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

public enum Cipher: Int {
    case NONE = 0
    @available(*, deprecated, message: "Use AES128GCMSIV instead. If set, will automatically switch to AES128GCMSIV.")
    case AES128GCM
    @available(*, deprecated, message: "Use AES256GCMSIV instead. If set, will automatically switch to AES256GCMSIV.")
    case AES256GCM
    case AES128GCMSIV
    case AES256GCMSIV
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
    var keepDays: Int = 7
    var maxSize: Int = 0
    var compress: CompressKind? = CompressKind.NONE
    var compressLevel: CompressLevel? = CompressLevel.DEFAULT
    var cipher: Cipher? = Cipher.NONE
    var cipherKey: [UInt8]? = []
    var cipherNonce: [UInt8]? = []
    var rotateHours: Int? = 24
    var extra: String?
    
    public init(
        level: Level,
        dirPath: String,
        name: String
    ) {
        self.level = level
        self.dirPath = dirPath
        self.name = name
    }
    
    public init(
        level: Level,
        dirPath: String,
        name: String,
        keepDays: Int,
        maxSize: Int,
        compress: CompressKind?,
        compressLevel: CompressLevel?,
        cipher: Cipher?,
        cipherKey: [UInt8]?,
        cipherNonce: [UInt8]?,
        rotateHours: Int?,
        extra: String?
    ) {
        self.level = level
        self.dirPath = dirPath
        self.name = name
        self.keepDays = keepDays
        self.maxSize = maxSize
        self.compress = compress ?? CompressKind.NONE
        self.compressLevel = compressLevel ?? CompressLevel.DEFAULT
        self.cipher = cipher ?? Cipher.NONE
        self.cipherKey = cipherKey ?? []
        self.cipherNonce = cipherNonce ?? []
        self.rotateHours = rotateHours ?? 24
        self.extra = extra
    }
    
}

public class EZLogConfigBuilder {
    
    var config: EZLogConfig
    
    public init(level: Level,
                dirPath: String,
                name: String) {
        config = EZLogConfig(level: level, dirPath: dirPath, name: name)
    }
    
    public func name(name: String) -> EZLogConfigBuilder {config.name = name; return self}
    public func level(level: Level) -> EZLogConfigBuilder {config.level = level; return self}
    public func dirPath(dirPath: String) -> EZLogConfigBuilder {config.dirPath = dirPath; return self}
    public func keepDays(keepDays: Int) -> EZLogConfigBuilder {config.keepDays = keepDays; return self}
    public func maxSize(maxSize: Int) -> EZLogConfigBuilder {config.maxSize = maxSize; return self}
    public func compress(compress: CompressKind? = CompressKind.NONE) -> EZLogConfigBuilder {config.compress = compress; return self}
    public func compressLevel(compressLevel: CompressLevel? = CompressLevel.DEFAULT) -> EZLogConfigBuilder {config.compressLevel = compressLevel; return self}
    public func cipherKey(cipherKey: [UInt8]? = []) -> EZLogConfigBuilder {config.cipherKey = cipherKey; return self}
    public func cipherNonce(cipherNonce: [UInt8]? = []) -> EZLogConfigBuilder {config.cipherNonce = cipherNonce; return self}
    public func rotateHours(rotateHours: Int? = 24) -> EZLogConfigBuilder {config.rotateHours = rotateHours; return self}
    public func extra(extra: String?) -> EZLogConfigBuilder {config.extra = extra; return self}
    
    public func cipher(cipher: Cipher? = Cipher.NONE) -> EZLogConfigBuilder {
        switch cipher {
        case .some(Cipher.AES128GCM):
            config.cipher = Cipher.AES128GCMSIV;
        case .some(Cipher.AES256GCM):
            config.cipher = Cipher.AES256GCMSIV;
        case nil:
            config.cipher = Cipher.NONE
        default:
            config.cipher = cipher;
        }
        return self
    }
    
    public func build() -> EZLogConfig {
        return config
    }
}

extension NSLock {
    
    @discardableResult
    func with<T>(_ block: () throws -> T) rethrows -> T {
        lock()
        defer { unlock() }
        return try block()
    }
}

public class EZCallback {
    let successClosure: (String, String, [String]) -> Void
    let failClosure: (String, String, String) -> Void
    public init(success: @escaping (String, String, [String]) -> Void,
                fail: @escaping (String, String, String) -> Void) {
        successClosure = success;
        failClosure = fail;
    }
}

var callbacks = Array<EZCallback>()
let callbackLock = NSLock()
var callbackRegister = false

let internalCallback: Callback = wrapCallback {name, date, logs in
    callbackLock.with {
        for callback in callbacks {
            callback.successClosure(name, date, logs)
        }
    }
} fail: { name, date, err in
    callbackLock.with {
        for callback in callbacks {
            callback.failClosure(name, date, err)
        }
    }
}

public func addCallback(callback: EZCallback) {
    callbackLock.with {
        if !callbackRegister {
            callbackRegister = true
            ezlog_register_callback(internalCallback)
        }
        callbacks.append(callback)
    }
}

public func removeCallback(callback: EZCallback) {
    callbackLock.with {
        removeCallbackNoLock(callback: callback)
    }
}

public func removeCallbackNoLock(callback: EZCallback) {
    for (index,item) in callbacks.enumerated() {
        if item === callback {
            callbacks.remove(at: index)
            break;
        }
    }
}
