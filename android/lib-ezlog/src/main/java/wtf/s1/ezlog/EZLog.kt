@file:Suppress("FunctionName", "unused")

package wtf.s1.ezlog

import java.text.SimpleDateFormat
import java.util.*
import java.util.concurrent.CopyOnWriteArrayList

object EZLog {

    enum class Cipher {
        NONE,

        @Deprecated(
            "AES256GCM is deprecated since 2.0, will auto replace with AES256GCMSIV",
            ReplaceWith("AES256GCMSIV"),
            DeprecationLevel.WARNING
        )
        AES256GCM,

        AES256GCMSIV,

        @Deprecated(
            "AES128GCM is deprecated since 2.0, will auto replace with AES128GCMSIV",
            ReplaceWith("AES128GCMSIV"),
            DeprecationLevel.WARNING
        )
        AES128GCM,

        AES128GCMSIV,
    }

    enum class Compress {
        NONE,
        ZLIB,
    }

    enum class CompressLevel {
        DEFAULT,
        BEST,
        FAST,
    }

    init {
        System.loadLibrary("ezlog")
    }

    const val VERBOSE = 5
    const val DEBUG = 4
    const val INFO = 3
    const val WARN = 2
    const val ERROR = 1
    const val Aes128Gcm = 1
    const val Aes256Gcm = 2
    const val Aes128GcmSiv = 3
    const val Aes256GcmSiv = 4
    const val CompressZlib = 1
    const val CompressDefault = 0
    const val CompressFast = 1
    const val CompressBest = 2

    @Volatile
    private var defaultLogger: EZLogger? = null

    @JvmStatic
    @Synchronized
    fun initWith(config: EZLogConfig) {
        nativeInit(config.enableTrace)
        defaultLogger = EZLogger(config)
    }

    @JvmStatic
    fun initNoDefault(enableTrace: Boolean) {
        nativeInit(enableTrace)
    }

    @JvmStatic
    fun v(tag: String?, msg: String?) {
        defaultLogger?.v(tag, msg)
    }

    @JvmStatic
    fun d(tag: String?, msg: String?) {
        defaultLogger?.d(tag, msg)
    }

    @JvmStatic
    fun i(tag: String?, msg: String?) {
        defaultLogger?.i(tag, msg)
    }

    @JvmStatic
    fun w(tag: String?, msg: String?) {
        defaultLogger?.w(tag, msg)
    }

    @JvmStatic
    fun e(tag: String?, msg: String?) {
        defaultLogger?.e(tag, msg)
    }

    @JvmStatic
    fun flush() {
        nativeFlushAll()
    }

    @JvmStatic
    @Deprecated("use EZLog.flush instead", ReplaceWith("flush"))
    fun _flush(logName: String?) {
        flush(logName)
    }

    @JvmStatic
    fun flush(logName: String?) {
        nativeFlush(logName)
    }

    @JvmStatic
    @Deprecated("use EZLog.trim instead", ReplaceWith("trim"))
    fun _trim() {
        trim()
    }

    @JvmStatic
    fun trim() {
        nativeTrim()
    }

    @JvmStatic
    @Deprecated("use EZLog.registerCallback instead", ReplaceWith("registerCallback"))
    fun _registerCallback(callback: EZLogCallback) {
        registerCallback(callback)
    }

    @JvmStatic
    fun registerCallback(callback: EZLogCallback) {
        addCallback(callback)
    }

    @JvmStatic
    fun requestLogFilesForDate(logName: String, start: Long, end: Long) {
        nativeRequestLogFilesForDate(logName, start, end)
    }

    @JvmStatic
    @Deprecated("use EZLog.requestLogFilesForDate instead", ReplaceWith("requestLogFilesForDate"))
    fun _requestLogFilesForDate(logName: String, date: Date) {
        requestLogFilesForDate(logName, date)
    }

    @JvmStatic
    fun requestLogFilesForDate(logName: String, date: Date) {
        requestLogFilesForDate(logName, date.time, date.time)
    }

    /**
     * create log from java
     *
     * @param config log config
     */
    @JvmStatic
    @Synchronized
    @Deprecated("use EZLog.createLogger instead", ReplaceWith("createLogger"))
    fun _createLogger(config: EZLogConfig) {
        createLogger(config)
    }

    @JvmStatic
    @Synchronized
    fun createLogger(config: EZLogConfig) {
        nativeCreateLogger(
            config.logName,
            config.maxLevel,
            config.dirPath,
            config.keepDays,
            config.compress,
            config.compressLevel,
            config.cipher,
            config.cipherKey,
            config.cipherNonce,
            config.rotateHours,
            config.extra
        )
    }

    @JvmStatic
    @Deprecated("use EZLog.log instead", ReplaceWith("log"))
    fun _log(logName: String, level: Int, target: String, logContent: String) {
        log(logName, level, target, logContent)
    }

    @JvmStatic
    fun log(logName: String, level: Int, target: String?, logContent: String?) {
        nativeLog(logName, level, target, logContent)
    }

    var callbacks = CopyOnWriteArrayList<EZLogCallback>()

    @Volatile
    var isRegister = false
    private var internalCallback: EZLogCallback? = null

    @JvmStatic
    @Synchronized
    fun addCallback(callback: EZLogCallback) {
        if (!isRegister) {
            isRegister = true
            internalCallback = object : EZLogCallback {
                override fun onSuccess(logName: String?, date: String?, logs: Array<String?>?) {
                    for (next in callbacks) {
                        next.onSuccess(logName, date, logs)
                    }
                }

                override fun onFail(logName: String?, date: String?, err: String?) {
                    for (next in callbacks) {
                        next.onFail(logName, date, err)
                    }
                }
            }
            nativeRegisterCallback(internalCallback)
        }
        if (!callbacks.contains(callback)) {
            callbacks.add(callback)
        }
    }

    @JvmStatic
    fun removeCallback(callback: EZLogCallback) {
        callbacks.remove(callback)
        // when callbacks size = 0, need unregister native callback.
    }

    /**
     * native init log library
     */
    @Synchronized
    private external fun nativeInit(enableTrace: Boolean)

    /**
     * native create a logger to print log
     *
     * @param logName       logger's name
     * @param maxLevel      max log out level
     * @param dirPath       log file in dir
     * @param keepDays      log live in days
     * @param compress      compress kind
     * @param compressLevel compress level
     * @param cipher        crypto kind
     * @param cipherKey     crypto key
     * @param cipherNonce   crypto nonce
     */
    private external fun nativeCreateLogger(
        logName: String,
        maxLevel: Int,
        dirPath: String,
        keepDays: Int,
        compress: Int,
        compressLevel: Int,
        cipher: Int,
        cipherKey: ByteArray,
        cipherNonce: ByteArray,
        rotateHours: Int,
        extra: String,
    )

    /**
     * native  print log to file, the log is associate by logName, filter by level
     *
     * @param logName    logger name
     * @param level      log level
     * @param target     log target
     * @param logContent log message
     */
    private external fun nativeLog(
        logName: String,
        level: Int,
        target: String?,
        logContent: String?
    )

    /**
     * native flush all logger, sync content to file
     */
    private external fun nativeFlushAll()

    /**
     * @param logName flush logger's name
     */
    private external fun nativeFlush(logName: String?)

    /**
     * @param callback log fetch callback
     */
    private external fun nativeRegisterCallback(callback: EZLogCallback?)

    /**
     * @param logName target log name
     * @param startTimestamp log create after the time
     * @param endTimeStamp log create before the time
     */
    private external fun nativeRequestLogFilesForDate(logName: String, startTimestamp: Long, endTimeStamp: Long)

    /**
     * trim log files
     */
    private external fun nativeTrim()
}