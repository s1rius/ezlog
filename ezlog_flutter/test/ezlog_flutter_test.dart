import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:ezlog_flutter/ezlog_flutter.dart';
import 'package:ezlog_flutter/ezlog_flutter_platform_interface.dart';
import 'package:ezlog_flutter/ezlog_flutter_method_channel.dart';
import 'package:plugin_platform_interface/plugin_platform_interface.dart';

class MockEZLogFlutterPlatform
    with MockPlatformInterfaceMixin
    implements EZLogFlutterPlatform {
  @override
  Future<String?> getPlatformVersion() => Future.value('42');

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
      Uint8List? cipherNonce) {
    // TODO: implement createLogger
  }

  @override
  void flush(String logName) {
    // TODO: implement flush
  }

  @override
  void init(bool enableTrace) {
    // TODO: implement init
  }

  @override
  void log(String logName, int level, String tag, String msg) {
    // TODO: implement log
  }

  @override
  Future<List<Object?>?> requestLogFilesForDate(String name, String date) {
    // TODO: implement requestLogFilesForDate
    throw UnimplementedError();
  }

  @override
  void flushAll() {
    // TODO: implement flushAll
  }
}

void main() {
  final EZLogFlutterPlatform initialPlatform = EZLogFlutterPlatform.instance;

  test('$MethodChannelEzlogFlutter is the default instance', () {
    expect(initialPlatform, isInstanceOf<MethodChannelEzlogFlutter>());
  });
}
