# Android ezlog

### Add ezlog to dependencies

Open top-level `build.gradle`, add `mavenCentral` to repositories.

```groovy
buildscript {
    repositories {
        ...
        mavenCentral()
        ...
    }
}

allprojects {
    repositories {
        ...
        mavenCentral()
        ...
    }
}
```

Open app level `build.gradle`, add `ezlog`

```groovy
dependencies {
    implementation "wtf.s1.ezlog:ezlog:0.1.7"
}
```

Sync gradle

### Setup in application

```kotlin
override fun onCreate() {
    super.onCreate()

    val path = File(filesDir, "ezlog").absolutePath
    val config = EZLogConfig.Builder("demo", path)
        .compress(EZLog.CompressZlib)
        .compressLevel(EZLog.CompressFast)
        .cipher(EZLog.Aes128Gcm)
        .cipherKey("a secret key!!!!".toByteArray())
        .cipherNonce("unique nonce".toByteArray())
        .enableTrace(BuildConfig.DEBUG)
        .build()
    EZLog.initWith(config)

    EZLog.v("ezlog", "first blood")

    EZLog.registerCallback(object : Callback {
        override fun onLogsFetchSuccess(
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
        }

        override fun onLogsFetchFail(logName: String?, date: String?, err: String?) {
            Log.i("ezlog", "$logName $date $err")
        }
    })
}

```