import 'dart:typed_data';

import 'ezlog_flutter_platform_interface.dart';

class EZLog {
  static void init(bool enableTrace) {
    EZLogFlutterPlatform.instance.init(enableTrace);
  }

  static void log(String logName, Level level, String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, level.id, tag, msg);
  }

  static void flushAll() {
    EZLogFlutterPlatform.instance.flushAll();
  }

  static Future<List<Object?>?> requestLogFilesForDate(
      String name, String date) {
    return EZLogFlutterPlatform.instance.requestLogFilesForDate(name, date);
  }

  static void trim() {
    EZLogFlutterPlatform.instance.trim();
  }
}

class EZLogConfig {
  String logName;
  int maxLevel;
  String dirPath;
  int keepDays;
  int compress = 0;
  int compressLevel = 0;
  int cipher = 0;
  Uint8List? cipherKey;
  Uint8List? cipherNonce;
  bool enableTrace = false;
  int rorateHours = 24;
  String? extra;

  EZLogConfig.plaintext(
      this.logName, this.maxLevel, this.dirPath, this.keepDays);

  EZLogConfig(
      this.logName,
      this.maxLevel,
      this.dirPath,
      this.keepDays,
      this.compress,
      this.compressLevel,
      this.cipher,
      this.cipherKey,
      this.cipherNonce,
      this.rorateHours,
      this.extra);
}

enum Level {
  /// Appropriate for error conditions.
  error,

  /// Appropriate for messages that are not error conditions,
  /// but more severe than notice.
  warning,

  /// Appropriate for informational messages.
  info,

  /// Appropriate for messages that contain information normally of use only when
  /// debugging a program.
  debug,

  /// Appropriate for messages that contain information normally of use only when
  /// tracing the execution of a program.
  trace,
}

extension LevelVal on Level {
  int get id {
    switch (this) {
      case Level.error:
        return 1;
      case Level.warning:
        return 2;
      case Level.info:
        return 3;
      case Level.debug:
        return 4;
      case Level.trace:
        return 5;
    }
  }
}

class EZLogger {
  String logName;

  EZLogger.createLog(EZLogConfig config) : logName = config.logName {
    EZLogFlutterPlatform.instance.createLogger(
        config.logName,
        config.maxLevel,
        config.dirPath,
        config.keepDays,
        config.compress,
        config.compressLevel,
        config.cipher,
        config.cipherKey,
        config.cipherNonce,
        config.rorateHours,
        config.extra);
  }

  void v(String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, Level.trace.id, tag, msg);
  }

  void d(String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, Level.debug.id, tag, msg);
  }

  void i(String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, Level.info.id, tag, msg);
  }

  void w(String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, Level.warning.id, tag, msg);
  }

  void e(String tag, String msg) {
    EZLogFlutterPlatform.instance.log(logName, Level.error.id, tag, msg);
  }

  void flush(String logName) {
    EZLogFlutterPlatform.instance.flush(logName);
  }

  void trim() {
    EZLogFlutterPlatform.instance.trim();
  }
}
