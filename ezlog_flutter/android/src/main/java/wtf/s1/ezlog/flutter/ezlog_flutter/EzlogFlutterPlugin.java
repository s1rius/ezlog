package wtf.s1.ezlog.flutter.ezlog_flutter;

import android.text.TextUtils;

import androidx.annotation.NonNull;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;

import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.MethodChannel.MethodCallHandler;
import io.flutter.plugin.common.MethodChannel.Result;
import wtf.s1.ezlog.EZLogCallback;
import wtf.s1.ezlog.EZLog;
import wtf.s1.ezlog.EZLogConfig;

/**
 * EzlogFlutterPlugin
 */
public class EzlogFlutterPlugin implements FlutterPlugin, MethodCallHandler {
    /// The MethodChannel that will the communication between Flutter and native Android
    ///
    /// This local reference serves to register the plugin with the Flutter Engine and unregister it
    /// when the Flutter Engine is detached from the Activity
    private MethodChannel channel;
    private final ResultHolder resultHolder = new ResultHolder();

    @Override
    public void onAttachedToEngine(@NonNull FlutterPluginBinding flutterPluginBinding) {
        channel = new MethodChannel(flutterPluginBinding.getBinaryMessenger(), "ezlog_flutter");
        channel.setMethodCallHandler(this);
    }

    @Override
    public void onMethodCall(@NonNull MethodCall call, @NonNull Result result) {
        if (call.method.equals("init")) {
            EZLog.initNoDefault(Boolean.TRUE.equals(call.arguments()));
        } else if (call.method.equals("createLogger")) {
            String name = call.argument("logName");
            String dirPath = call.argument("dirPath");

            if (TextUtils.isEmpty(name) || TextUtils.isEmpty(dirPath)) {
                return;
            }

            EZLogConfig.Builder builder = new EZLogConfig.Builder(name, dirPath);
            EZLogConfig config = builder.maxLevel(argumentOrDefault(call, "maxLevel", EZLog.ERROR))
                    .keepDays(argumentOrDefault(call, "keepDays", 7))
                    .compress(argumentOrDefault(call, "compress", EZLog.CompressZlib))
                    .compressLevel(argumentOrDefault(call, "compressLevel", EZLog.CompressDefault))
                    .cipher(argumentOrDefault(call, "cipher", 0))
                    .cipherKey(argumentOrDefault(call, "cipherKey", new byte[]{}))
                    .cipherNonce(argumentOrDefault(call, "cipherNonce", new byte[]{}))
                    .rotateHours(argumentOrDefault(call, "rotateHours", 24))
                    .build();
            EZLog._createLogger(config);

        } else if (call.method.equals("log")) {
            String name = argumentOrDefault(call, "logName", "");
            int level = argumentOrDefault(call, "level", EZLog.VERBOSE);
            String tag = argumentOrDefault(call, "tag", "");
            String msg = argumentOrDefault(call, "msg", "");

            if (TextUtils.isEmpty(name) || TextUtils.isEmpty(msg)) {
                return;
            }

            EZLog._log(name, level, tag, msg);
            result.success(null);
        } else if (call.method.equals("flush")) {
            String name = argumentOrDefault(call, "logName", "");
            if (TextUtils.isEmpty(name)) {
                return;
            }
            EZLog._flush(name);
            result.success(null);
        } else if (call.method.equals("requestLogFilesForDate")) {
            String name = argumentOrDefault(call, "logName", "");
            String date = argumentOrDefault(call, "date", "");
            resultHolder.update(name, result);
            resultHolder.bind();
            EZLog._requestLogFilesForDate(name, date);
        } else if (call.method.equals("flush")) {
            EZLog.flush();
            result.success(null);
        } else if (call.method.equals("trim")) {
            EZLog._trim();
        } else {
            result.notImplemented();
        }
    }

    @Override
    public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
        channel.setMethodCallHandler(null);
        resultHolder.unbind();
    }


    public static <T> T argumentOrDefault(@NonNull MethodCall call, @NonNull String key, T obj) {
        if (call.hasArgument(key)) {
            try {
                T value = call.argument(key);
                if (value != null) {
                    return value;
                }
            } catch (ClassCastException e) {
                e.printStackTrace();
            }

        }
        return obj;
    }

    private static class ResultHolder implements EZLogCallback {
        public HashMap<String, Result> result = new HashMap<String, Result>();
        private volatile boolean isBind = false;

        public ResultHolder() {
        }

        public void update(String logName, Result result) {
            this.result.put(logName, result);
        }

        public synchronized void bind() {
            if (!isBind) {
                isBind = true;
                EZLog.addCallback(this);
            }
        }

        public synchronized void unbind() {
            EZLog.removeCallback(this);
        }

        @Override
        public void onSuccess(String logName, String date, String[] logs) {
            Result r = this.result.remove(logName);
            if (r != null) {
                r.success(Arrays.asList(logs));
            }
        }

        @Override
        public void onFail(String logName, String date, String err) {
            Result r = this.result.remove(logName);
            if (r != null) {
                r.success(null);
            }
        }
    }
}
