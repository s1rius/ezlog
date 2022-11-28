package wtf.s1.ezlog

import wtf.s1.ezlog.EZLog.DEBUG
import wtf.s1.ezlog.EZLog.ERROR
import wtf.s1.ezlog.EZLog.INFO
import wtf.s1.ezlog.EZLog.VERBOSE
import wtf.s1.ezlog.EZLog.WARN
import wtf.s1.ezlog.EZLog.createLogger
import wtf.s1.ezlog.EZLog.flush
import wtf.s1.ezlog.EZLog.log

class EZLogger(config: EZLogConfig) {
    private val loggerName: String

    init {
        loggerName = config.logName
        createLogger(config)
    }

    fun v(tag: String, msg: String) {
        log(loggerName, VERBOSE, tag, msg)
    }

    fun d(tag: String, msg: String) {
        log(loggerName, DEBUG, tag, msg)
    }

    fun i(tag: String, msg: String) {
        log(loggerName, INFO, tag, msg)
    }

    fun w(tag: String, msg: String) {
        log(loggerName, WARN, tag, msg)
    }

    fun e(tag: String, msg: String) {
        log(loggerName, ERROR, tag, msg)
    }

    fun flush() {
        flush(loggerName)
    }
}