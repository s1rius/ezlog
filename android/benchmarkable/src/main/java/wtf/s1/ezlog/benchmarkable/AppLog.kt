package wtf.s1.ezlog.benchmarkable

import android.content.Context
import android.text.TextUtils
import com.dianping.logan.Logan
import com.dianping.logan.LoganConfig
import com.dianping.logan.SendLogRunnable
import com.dianping.logan.Util
import com.tencent.mars.xlog.Log
import com.tencent.mars.xlog.Xlog
import wtf.s1.ezlog.EZLog
import wtf.s1.ezlog.EZLogCallback
import wtf.s1.ezlog.EZLogConfig
import wtf.s1.ezlog.EZLogger
import java.io.File
import java.util.*


abstract class AppLog {

    abstract fun init()

    abstract fun v(tag: String, msg: String)

    abstract fun d(tag: String, msg: String)

    abstract fun i(tag: String, msg: String)

    abstract fun w(tag: String, msg: String)

    abstract fun e(tag: String, msg: String)

    abstract fun flush()
    abstract fun requestLog(date: Date)
    abstract fun registerCallback()
}

class AppEZLog(private val config: EZLogConfig): AppLog() {

    lateinit var log: EZLogger

    override fun init() {
        EZLog.initNoDefault(BuildConfig.DEBUG)
        log = EZLogger(config)
    }

    override fun v(tag: String, msg: String) {
        log.v(tag, msg)
    }

    override fun d(tag: String, msg: String) {
        log.d(tag, msg)
    }

    override fun i(tag: String, msg: String) {
        log.i(tag, msg)
    }

    override fun w(tag: String, msg: String) {
        log.w(tag, msg)
    }

    override fun e(tag: String, msg: String) {
        log.e(tag, msg)
    }

    override fun flush() {
        log.flush()
    }

    override fun requestLog(date: Date) {
        EZLog.requestLogFilesForDate(config.logName, date)
    }

    override fun registerCallback() {
        EZLog.registerCallback(callback)
    }

    private val callback = object : EZLogCallback {

        override fun onSuccess(logName: String?, date: String?, logs: Array<String?>?) {
            android.util.Log.i("ezlog", "$logName $date ${logs.contentToString()}")
            logs?.let {
                logs.getOrNull(0)?.let { log ->
                    android.util.Log.i("ezlog", "$log exists = ${File(log).exists()}")
                }
            }
            EZLog.trim()
        }

        override fun onFail(logName: String?, date: String?, err: String?) {
            android.util.Log.i("ezlog", "$logName $date $err")
            EZLog.trim()
        }
    }

}

class AppLogan(private val config: LoganConfig): AppLog() {


    override fun init() {
        Logan.init(config)
    }

    override fun v(tag: String, msg: String) {
        Logan.w("$tag:$msg", android.util.Log.VERBOSE)
    }

    override fun d(tag: String, msg: String) {
        Logan.w("$tag:$msg", android.util.Log.DEBUG)
    }

    override fun i(tag: String, msg: String) {
        Logan.w("$tag:$msg", android.util.Log.INFO)
    }

    override fun w(tag: String, msg: String) {
        Logan.w("$tag:$msg", android.util.Log.VERBOSE)
    }

    override fun e(tag: String, msg: String) {
        Logan.w("$tag:$msg", android.util.Log.ERROR)
    }

    override fun flush() {
        Logan.f()
    }

    override fun requestLog(date: Date) {
        Logan.s(arrayOf(Util.getDateStr(date.time)), object: SendLogRunnable() {
            override fun sendLog(logFile: File?) {
                logFile?.let {
                    Log.i("logan", it.path)
                }
            }
        })
    }

    override fun registerCallback() {
        // noting to do
    }

}


class AppXLog(val context: Context): AppLog() {

    lateinit var logPath: String
    lateinit var cachePath: String
    override fun init() {
        System.loadLibrary("c++_shared");
        System.loadLibrary("marsxlog");

        logPath = File(context.filesDir, "/xlog").absolutePath

        // this is necessary, or may crash for SIGBUS
        cachePath = File(context.filesDir, "/xlog/cache").absolutePath

        //init xlog
        val xlog = Xlog()
        Log.setLogImp(xlog);

        if (BuildConfig.DEBUG) {
            Log.setConsoleLogOpen(true)
            Log.appenderOpen(Xlog.LEVEL_DEBUG, Xlog.AppednerModeAsync, cachePath, logPath, "xl", 1)
        } else {
            Log.setConsoleLogOpen(false)
            Log.appenderOpen(Xlog.LEVEL_INFO, Xlog.AppednerModeAsync, cachePath, logPath, "xl", 1)
        }
    }

    override fun v(tag: String, msg: String) {
        Log.v(tag, msg)
    }

    override fun d(tag: String, msg: String) {
        Log.d(tag, msg)
    }

    override fun i(tag: String, msg: String) {
        Log.i(tag, msg)
    }

    override fun w(tag: String, msg: String) {
        Log.w(tag, msg)
    }

    override fun e(tag: String, msg: String) {
        Log.e(tag, msg)
    }

    override fun flush() {
        Log.appenderFlush()
    }

    override fun requestLog(date: Date) {
        //noting to do
    }

    override fun registerCallback() {
        //noting to do
    }

}