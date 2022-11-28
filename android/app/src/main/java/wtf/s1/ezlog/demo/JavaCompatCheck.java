package wtf.s1.ezlog.demo;

import java.util.Date;

import wtf.s1.ezlog.EZLog;
import wtf.s1.ezlog.EZLogConfig;
import wtf.s1.ezlog.EZLogger;

public class JavaCompatCheck {

    public static void check() {
        EZLog._createLogger(null);
        EZLog.initNoDefault(false);
        EZLog.initWith(null);
        EZLog.i(null, null);
        EZLog.d(null, null);
        EZLog.v(null, null);
        EZLog.w(null, null);
        EZLog.e(null, null);
        EZLog._flush("");
        EZLog.flush();
        EZLog._requestLogFilesForDate(null, (Date) null);
        EZLog._requestLogFilesForDate(null, "");
        EZLog._registerCallback(null);
        EZLog.addCallback(null);
        EZLog.removeCallback(null);
        EZLog._trim();

        EZLogConfig config = new EZLogConfig.Builder("", null)
                .cipher(EZLog.Aes128Gcm)
                .cipherKey(new byte[0])
                .cipherNonce(new byte[0])
                .compress(EZLog.CompressZlib)
                .compressLevel(EZLog.CompressBest)
                .build();

        EZLogger logger = new EZLogger(config);
        logger.d("", null);
        logger.flush();

    }
}
