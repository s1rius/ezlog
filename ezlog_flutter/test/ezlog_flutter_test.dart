import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:ezlog_flutter/ezlog_flutter_platform_interface.dart';
import 'package:ezlog_flutter/ezlog_flutter_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockEZLogFlutterPlatform
    with MockPlatformInterfaceMixin
    implements EZLogFlutterPlatform {
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
      int rotateHours,
      String? extra) {
    throw UnimplementedError();
  }

  @override
  void flush(String logName) {
    throw UnimplementedError();
  }

  @override
  void init(bool enableTrace) {
    throw UnimplementedError();
  }

  @override
  void log(String logName, int level, String tag, String msg) {
    throw UnimplementedError();
  }

  @override
  Future<List<Object?>?> requestLogFilesForDate(String name, String date) {
    throw UnimplementedError();
  }

  @override
  void flushAll() {
    throw UnimplementedError();
  }

  @override
  void trim() {
    throw UnimplementedError();
  }
}

void main() {
  final EZLogFlutterPlatform initialPlatform = EZLogFlutterPlatform.instance;

  test('$MethodChannelEzlogFlutter is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelEzlogFlutter>());
  });
}
