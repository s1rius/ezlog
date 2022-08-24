package wtf.s1.ezlog;

import org.jetbrains.annotations.NotNull;

import java.util.concurrent.CopyOnWriteArrayList;

public class EZLog {
    static {
        System.loadLibrary("ezlog");
    }
    public static final int VERBOSE = 5;
    public static final int DEBUG = 4;
    public static final int INFO = 3;
    public static final int WARN = 2;
    public static final int ERROR = 1;

    public static final int Aes128Gcm = 1;
    public static final int Aes256Gcm = 2;

    public static final int CompressZlib = 1;

    public static final int CompressDefault = 0;
    public static final int CompressFast = 1;
    public static final int CompressBest = 2;

    private static volatile EZLogger defaultLogger;

    public static synchronized void initWith(@NotNull EZLogConfig config) {
        init(config.enableTrace);
        defaultLogger = new EZLogger(config);
    }

    public static void initNoDefault(boolean enableTrace) {
        init(enableTrace);
    }

    public static void v (String tag, String msg) {
        if (defaultLogger != null) {
            defaultLogger.v(tag, msg);
        }
    }
    public static void d (String tag, String msg) {
        if (defaultLogger != null) {
            defaultLogger.d(tag, msg);
        }
    }
    public static void i (String tag, String msg) {
        if (defaultLogger != null) {
            defaultLogger.i(tag, msg);
        }
    }
    public static void w (String tag, String msg) {
        if (defaultLogger != null) {
            defaultLogger.w(tag, msg);
        }
    }
    public static void e (String tag, String msg) {
        if (defaultLogger != null) {
            defaultLogger.e(tag, msg);
        }
    }

    public static void flush() {
        flushAll();
    }

    public static void _flush(String logName) {
        flush(logName);
    }

    private static void _registerCallback(Callback callback) {
        addCallback(callback);
    }

    public static void _requestLogFilesForDate(String logName, String date) {
        requestLogFilesForDate(logName, date);
    }

    /**
     * create log from java
     * @param config log config
     */
    public synchronized static void _createLogger(@NotNull EZLogConfig config) {
        createLogger(config.logName,
                config.maxLevel,
                config.dirPath,
                config.keepDays,
                config.compress,
                config.compressLevel,
                config.cipher,
                config.cipherKey,
                config.cipherNonce
        );
    }

    public static void _log(String logName, int level, String target, String logContent) {
        log(logName, level, target, logContent);
    }

    static CopyOnWriteArrayList<Callback> callbacks = new CopyOnWriteArrayList<>();
    volatile static boolean isRegister = false;
    public static synchronized void addCallback(@NotNull Callback callback) {
        if (!isRegister) {
            isRegister = true;
            registerCallback(new Callback() {
                @Override
                public void onLogsFetchSuccess(String logName, String date, String[] logs) {
                    for (Callback next : callbacks) {
                        next.onLogsFetchSuccess(logName, date, logs);
                    }
                }

                @Override
                public void onLogsFetchFail(String logName, String date, String err) {
                    for (Callback next : callbacks) {
                        next.onLogsFetchFail(logName, date, err);
                    }
                }
            });
        }
        callbacks.add(callback);
    }

    public static void removeCallback(@NotNull Callback callback) {
        callbacks.remove(callback);
        // when callbacks size = 0, need unregister native callback.
    }

    /**
     * native init log library
     */
    private static synchronized native void init(boolean enableTrace);

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
    private static native void createLogger(
            String logName,
            int maxLevel,
            String dirPath,
            int keepDays,
            int compress,
            int compressLevel,
            int cipher,
            byte[] cipherKey,
            byte[] cipherNonce
    );

    /**
     * native  print log to file, the log is associate by logName, filter by level
     *
     * @param logName    logger name
     * @param level      log level
     * @param target     log target
     * @param logContent log message
     */
    private static native void log(String logName, int level, String target, String logContent);

    /**
     * native flush all logger, sync content to file
     */
    private static native void flushAll();

    /**
     *
     * @param logName flush logger's name
     */
    private static native void flush(String logName);

    /**
     * @param callback log fetch callback
     */
    private static native void registerCallback(Callback callback);

    /**
     *
     * @param logName target log name
     * @param date target log date
     */
    private static native void requestLogFilesForDate(String logName, String date);
}