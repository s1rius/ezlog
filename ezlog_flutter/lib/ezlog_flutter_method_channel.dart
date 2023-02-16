import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'ezlog_flutter_platform_interface.dart';

/// An implementation of [EZLogFlutterPlatform] that uses method channels.
class MethodChannelEzlogFlutter extends EZLogFlutterPlatform {
  /// The method channel used to interact with the native platform.
  @visibleForTesting
  final methodChannel = const MethodChannel('ezlog_flutter');

  @override
  void init(bool enableTrace) {
    methodChannel.invokeMethod<bool>('init', enableTrace);
  }

  @override
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
      int rotateHours) {
    methodChannel.invokeMethod('createLogger', <String, dynamic>{
      "logName": logName,
      "maxLevel": maxLevel,
      "dirPath": dirPath,
      "keepDays": keepDays,
      "compress": compress,
      "compressLevel": compressLevel,
      "cipher": cipher,
      "cipherKey": cipherKey,
      "cipherNonce": cipherNonce,
      "rotateHours": rotateHours,
    });
  }

  @override
  void log(String logName, int level, String tag, String msg) {
    methodChannel.invokeMethod("log", <String, dynamic>{
      "logName": logName,
      "level": level,
      "tag": tag,
      "msg": msg,
    });
  }

  @override
  void flush(String logName) {
    methodChannel.invokeMethod<String>("flush", logName);
  }

  @override
  void flushAll() {
    methodChannel.invokeMapMethod("flushAll");
  }

  @override
  Future<List<Object?>?> requestLogFilesForDate(String name, String date) {
    return methodChannel
        .invokeMethod("requestLogFilesForDate", <String, dynamic>{
      "logName": name,
      "date": date,
    });
  }
}
