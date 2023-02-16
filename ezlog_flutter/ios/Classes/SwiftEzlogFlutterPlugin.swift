import Flutter
import UIKit
import EZLog

var resultDict: [String: FlutterResult] = [:]

public class SwiftEzlogFlutterPlugin: NSObject, FlutterPlugin {
    
    let callback = EZCallback { name, d, logs in
        let result = resultDict.removeValue(forKey: name)
        if result != nil {
            result!(Array(logs))
        }
    } fail: { name, date, err in
        let result = resultDict.removeValue(forKey: name)
        if result != nil {
            result!(FlutterError(code: "request error", message: err, details: nil))
        }
    }
    private var lock = NSLock()
    private var callbackRegister = false

    
    public static func register(with registrar: FlutterPluginRegistrar) {
        let channel = FlutterMethodChannel(name: "ezlog_flutter", binaryMessenger: registrar.messenger())
        let instance = SwiftEzlogFlutterPlugin()
        registrar.addMethodCallDelegate(instance, channel: channel)
    }
    
    public func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
        switch call.method {
        case "init":
            let enableTrace = call.arguments as? Bool ?? false
            ezlog_init(enableTrace)
            result(nil)
            break
        case "createLogger":
            let arguments = call.arguments as? [String: Any] ?? [String: Any]()
            let logName = arguments["logName"] as? String ?? ""
            let dirPath = arguments["dirPath"] as? String ?? ""
            if logName.isEmpty || dirPath.isEmpty {
                break
            }
            
            let maxLevel = Level(rawValue: arguments["maxLevel"] as? Int ?? 0) ?? Level.trace
            let keepDays = arguments["keepDays"] as? Int ?? 7
            
            let compress = CompressKind(rawValue: (arguments["compress"] as? Int ?? 0))
            let compressLevel = CompressLevel(rawValue: arguments["compressLevel"] as? Int ?? 0)
            let cipher = Cipher(rawValue: arguments["cipher"] as? Int ?? 0)
            let cipherKey = arguments["cipherKey"] as? [UInt8] ?? []
            let cipherNonce = arguments["cipherNonce"] as? [UInt8] ?? []
            let rotateHours = arguments["rotateHours"] as? Int ?? 24
            let config = EZLogConfig(level: maxLevel,
                                     dirPath: dirPath,
                                     name: logName,
                                     keepDays: keepDays,
                                     maxSize: 150*1024,
                                     compress: compress,
                                     compressLevel: compressLevel,
                                     cipher:cipher,
                                     cipherKey:cipherKey,
                                     cipherNonce:cipherNonce,
                                     rotateHours: rotateHours)
            var _ = EZLogger(config: config)
            result(nil)
            break
        case "log":
            let arguments = call.arguments as? [String: Any] ?? [String: Any]()
            let logName = arguments["logName"] as? String ?? ""
            let level = Level(rawValue: arguments["level"] as? Int ?? 0) ?? Level.trace
            let tag = arguments["tag"] as? String ?? ""
            let msg = arguments["msg"] as? String ?? ""
            if logName.isEmpty || msg.isEmpty {
                break
            }
            log(logName: logName, level: level, tag: tag, msg: msg)
            result(nil)
            break
        case "flush":
            let arguments = call.arguments as? [String: Any] ?? [String: Any]()
            let logName = arguments["logName"] as? String ?? ""
            flush(logName:logName)
            result(nil)
            break
        case "flushAll":
            flushAll()
            result(nil)
            break
        case "requestLogFilesForDate":
            lock.withLock {
                if !callbackRegister {
                    callbackRegister = true
                    addCallback(callback:callback)
                }
            }
            let arguments = call.arguments as? [String: Any] ?? [String: Any]()
            let logName = arguments["logName"] as? String ?? ""
            let date = arguments["date"] as? String ?? ""
            resultDict[logName] = result
            requestLogsForDate(logName:logName, date:date)
            break
        case "trim":
            trim()
            break
        default:
            result(nil)
        }
    }
}

