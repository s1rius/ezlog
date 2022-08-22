import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:ezlog_flutter/ezlog_flutter_method_channel.dart';

void main() {
  MethodChannelEzlogFlutter platform = MethodChannelEzlogFlutter();
  const MethodChannel channel = MethodChannel('ezlog_flutter');

  TestWidgetsFlutterBinding.ensureInitialized();

  setUp(() {
    channel.setMockMethodCallHandler((MethodCall methodCall) async {
      return '42';
    });
  });

  tearDown(() {
    channel.setMockMethodCallHandler(null);
  });
}
