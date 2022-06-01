package wtf.s1.ezlog.demo

import android.app.Activity
import android.app.Application
import android.os.Bundle
import wtf.s1.ezlog.EZLog
import wtf.s1.ezlog.EZLogConfig
import java.io.File

class DebugApp : Application() {

    override fun onCreate() {
        super.onCreate()

        val path = File(filesDir, "ezlog").absolutePath
        val config = EZLogConfig.Builder("demo", path)
            .compress(EZLog.Compress_Zlib)
            .compressLevel(EZLog.Compress_Fast)
            .cipher(EZLog.Aes128Gcm)
            .cipherKey("a secret key!!!!".toByteArray())
            .cipherNonce("unique nonce".toByteArray())
            .build()
        EZLog.initWith(config)

        registerActivityLifecycleCallbacks(object : ActivityLifecycleCallbacks {
            override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) {
                EZLog.v(activity.localClassName, "onCreate")
            }

            override fun onActivityStarted(activity: Activity) {
                EZLog.v(activity.localClassName, "onStart")
            }

            override fun onActivityResumed(activity: Activity) {
                EZLog.v(activity.localClassName, "onResume")
            }

            override fun onActivityPaused(activity: Activity) {
                EZLog.v(activity.localClassName, "onPause")
            }

            override fun onActivityStopped(activity: Activity) {
                EZLog.v(activity.localClassName, "onStop")
            }

            override fun onActivityDestroyed(activity: Activity) {
                EZLog.v(activity.localClassName, "onDestory")
            }

            override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) {
                EZLog.v(activity.localClassName, "onSaveInstance")
            }
        })
    }
}
