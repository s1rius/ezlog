package wtf.s1.ezlog.benchmarkable

import android.util.Log
import java.util.*
import kotlin.random.Random
import kotlin.random.nextInt

class LogController(
    private val log: AppLog,
    private val logLength: Int = 100,
    private val logCount: Int = 1000
) {

    private val str =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789,.:;!@#$%^&*()_+-"
    private val tag = "test"


    fun init() {
        log.init()
    }

    fun bulkLog() {
        for (i in 0 until logCount) {
            log()
        }
    }

    fun log() {
        val sb = StringBuilder()
        for (i in 1..logLength) {
            sb.append(str[Random.nextInt(0, str.length)])
        }
        when (Random.nextInt(Log.VERBOSE..Log.ERROR)) {
            Log.VERBOSE -> log.v(tag, sb.toString())
            Log.DEBUG -> log.d(tag, sb.toString())
            Log.INFO -> log.i(tag, sb.toString())
            Log.WARN -> log.w(tag, sb.toString())
            Log.ERROR -> log.e(tag, sb.toString())
        }
    }

    fun flush() {
        log.flush()
    }

    fun requestLog(date: Date) {
        log.requestLog(date)
    }

    fun registerCallback() {
        log.registerCallback()
    }


}