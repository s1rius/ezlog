package wtf.s1.ezlog;

import org.jetbrains.annotations.NotNull;

import static wtf.s1.ezlog.EZLog.VERBOSE;
import static wtf.s1.ezlog.EZLog.DEBUG;
import static wtf.s1.ezlog.EZLog.INFO;
import static wtf.s1.ezlog.EZLog.WARN;
import static wtf.s1.ezlog.EZLog.ERROR;

public class EZLogger {

    private @NotNull final String loggerName;

    public EZLogger(EZLogConfig config) {
        this.loggerName = config.logName;
        EZLog._createLogger(config);
    }

    public void v(String tag, String msg) {
        EZLog._log(loggerName, VERBOSE, tag, msg);
    }

    public void d(String tag, String msg) {
        EZLog._log(loggerName, DEBUG, tag, msg);
    }

    public void i(String tag, String msg) {
        EZLog._log(loggerName, INFO, tag, msg);
    }

    public void w(String tag, String msg) {
        EZLog._log(loggerName, WARN, tag, msg);
    }

    public void e(String tag, String msg) {
        EZLog._log(loggerName, ERROR, tag, msg);
    }


}
