package ezlog;

import androidx.test.runner.AndroidJUnit4;
import org.jetbrains.annotations.Nullable;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import wtf.s1.ezlog.EZLog;
import wtf.s1.ezlog.EZLogCallback;
import wtf.s1.ezlog.EZLogConfig;
import wtf.s1.ezlog.EZLogger;

import java.util.Date;

@RunWith(AndroidJUnit4.class)
public class TestJavaCompat {

    @Before
    public void init() {
        EZLog.initNoDefault(true);
    }

    @Test
    public void testCheckV1Method() {
        EZLog._createLogger(new EZLogConfig("default", ""));
        EZLog._trim();
        EZLog._registerCallback(new EZLogCallback() {
            @Override
            public void onSuccess(@Nullable String logName, @Nullable String date, @Nullable String[] logs) {

            }

            @Override
            public void onFail(@Nullable String logName, @Nullable String date, @Nullable String err) {

            }
        });
        EZLog._requestLogFilesForDate("", new Date());
        EZLog._flush("");
    }

    @Test
    public void testEZLogJavaCompat() {
        EZLogConfig config = new EZLogConfig.Builder("", "")
                .cipher(EZLog.Aes128Gcm)
                .cipherKey(new byte[0])
                .cipherNonce(new byte[0])
                .compress(EZLog.CompressZlib)
                .compressLevel(EZLog.CompressBest)
                .build();
        EZLog.initNoDefault(false);
        EZLog.initWith(config);
        EZLog.i(null, null);
        EZLog.d(null, null);
        EZLog.v(null, null);
        EZLog.w(null, null);
        EZLog.e(null, null);

        EZLog.flush();

        EZLogCallback callback = new EZLogCallback() {
            @Override
            public void onSuccess(@Nullable String logName, @Nullable String date, @Nullable String[] logs) {

            }

            @Override
            public void onFail(@Nullable String logName, @Nullable String date, @Nullable String err) {

            }
        };

        EZLog.addCallback(callback);
        EZLog.removeCallback(callback);

        EZLogger logger = new EZLogger(config);
        logger.d("main", "log info 12345");
        logger.flush();
    }
}
