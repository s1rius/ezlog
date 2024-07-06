import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const MethodChannel channel = MethodChannel('ezlog_flutter');

  TestWidgetsFlutterBinding.ensureInitialized();

  Future<void> onMethodCall(MethodCall call) {
    switch (call.method) {
      default:
        throw UnimplementedError(
            "${call.method} was invoked but isn't implemented by PlatformViewsService");
    }
  }

  setUp(() {
    channel.setMethodCallHandler(onMethodCall);
  });

  tearDown(() {
    // ignore: deprecated_member_use
    channel.setMockMethodCallHandler(null);
  });
}
