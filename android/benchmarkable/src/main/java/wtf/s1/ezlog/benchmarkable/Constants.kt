package wtf.s1.ezlog.benchmarkable

import android.content.Context
import com.dianping.logan.LoganConfig
import wtf.s1.ezlog.EZLog
import wtf.s1.ezlog.EZLogConfig
import java.io.File

fun ezlogDir(context: Context): String {
    return File(context.filesDir, "ezlog").absolutePath
}

fun ezlogDemoConfig(context: Context): EZLogConfig {
    return EZLogConfig.Builder("demo", ezlogDir(context))
        .compress(EZLog.Compress.ZLIB)
        .compressLevel(EZLog.CompressLevel.BEST)
        .cipher(EZLog.Cipher.AES128GCMSIV)
        .cipherKey("a secret key!!!!".toByteArray())
        .cipherNonce("unique nonce".toByteArray())
        .extra("extra info")
        .keepDays(10)
        .enableTrace(true)
        .build()
}

fun uiConfig(context: Context): EZLogConfig {
    return EZLogConfig.Builder("ui", ezlogDir(context))
        .cipher(EZLog.Cipher.AES256GCMSIV)
        .cipherKey("a secret key!!!!".toByteArray())
        .cipherNonce("unique nonce".toByteArray())
        .build()
}

fun loganConfig(context: Context): LoganConfig {
    return LoganConfig.Builder()
        .setCachePath(
            context.applicationContext.cacheDir.absolutePath
        )
        .setPath(
            (context.applicationContext.filesDir
                .absolutePath
                    + File.separator) + "logan_v1"
        )
        .setEncryptKey16("0123456789012345".toByteArray())
        .setEncryptIV16("0123456789012345".toByteArray())
        .build()
}


