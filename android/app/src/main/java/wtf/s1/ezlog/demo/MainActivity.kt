package wtf.s1.ezlog.demo

import android.os.Bundle
import android.util.Log
import android.view.View
import androidx.appcompat.app.AppCompatActivity
import wtf.s1.ezlog.EZLog
import wtf.s1.ezlog.EZLogCallback
import wtf.s1.ezlog.benchmarkable.LogController
import wtf.s1.ezlog.benchmarkable.AppEZLog
import wtf.s1.ezlog.benchmarkable.ezlogDemoConfig
import java.io.File

class MainActivity : AppCompatActivity() {

    lateinit var logController: LogController

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        findViewById<View>(R.id.init).setOnClickListener {
            logController.init()
        }

        findViewById<View>(R.id.log).setOnClickListener {
            logController.log()
        }

        findViewById<View>(R.id.logs).setOnClickListener {
            logController.bulkLog()
        }

        findViewById<View>(R.id.flush).setOnClickListener {
            logController.flush()
        }

        findViewById<View>(R.id.get_files).setOnClickListener {
            callback()
            requestLog()
        }

        // logController = LogController(AppEZLog(ezlogDemoConfig(this)))
    }

    private fun callback() {
        EZLog._registerCallback(object : EZLogCallback {
            override fun onSuccess(
                logName: String?,
                date: String?,
                logs: Array<out String>?
            ) {
                Log.i("ezlog", "$logName $date ${logs.contentToString()}")
                logs?.let {
                    logs.getOrNull(0)?.let { log ->
                        Log.i("ezlog", "check file exists ${File(log).exists()}")
                    }
                }
                EZLog._trim()
            }

            override fun onFail(logName: String?, date: String?, err: String?) {
                Log.i("ezlog", "$logName $date $err")
                EZLog._trim()
            }
        })
    }

    private fun requestLog() {
        Thread({
            EZLog.v("ChildThread", "run on background")
            Thread.sleep(1000)
            EZLog._requestLogFilesForDate("demo", "2022_06_19")
        }, "background_log").start()
    }
}
