import 'dart:typed_data';

import 'package:plugin_platform_interface/plugin_platform_interface.dart';

import 'ezlog_flutter_method_channel.dart';

abstract class EZLogFlutterPlatform extends PlatformInterface {
  /// Constructs a EzlogFlutterPlatform.
  EZLogFlutterPlatform() : super(token: _token);

  static final Object _token = Object();

  static EZLogFlutterPlatform _instance = MethodChannelEzlogFlutter();

  /// The default instance of [EZLogFlutterPlatform] to use.
  ///
  /// Defaults to [MethodChannelEzlogFlutter].
  static EZLogFlutterPlatform get instance => _instance;

  /// Platform-specific implementations should set this with their own
  /// platform-specific class that extends [EZLogFlutterPlatform] when
  /// they register themselves.
  static set instance(EZLogFlutterPlatform instance) {
    PlatformInterface.verifyToken(instance, _token);
    _instance = instance;
  }

  void createLogger(
      String logName,
      int maxLevel,
      String dirPath,
      int keepDays,
      int compress,
      int compressLevel,
      int cipher,
      Uint8List? cipherKey,
      Uint8List? cipherNonce,
      int rotateHours,
      String? extra) {
    throw UnimplementedError('createLogger() has not been implemented.');
  }

  void init(bool enableTrace) {
    throw UnimplementedError('init() has not been implemented.');
  }

  void log(String logName, int level, String tag, String msg) {
    throw UnimplementedError('log() has not been implemented.');
  }

  void flush(String logName) {
    throw UnimplementedError('flush() has not been implemented.');
  }

  void flushAll() {
    throw UnimplementedError('flush() has not been implemented.');
  }

  Future<List<Object?>?> requestLogFilesForDate(String name, String date) {
    throw UnimplementedError(
        'requestLogFilesForDate() has not been implemented.');
  }

  void trim() {
    throw UnimplementedError('trimAll() has not been implemented.');
  }
}
