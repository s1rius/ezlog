# Flutter ezlog

### Add ezlog_flutter as a dependency in your pubspec.yaml file.

```yaml
dependencies:
  ezlog_flutter: ^0.2.0
```

### Example

```dart
import 'dart:io';
import 'package:flutter/material.dart';
import 'dart:async';
import 'package:ezlog_flutter/ezlog_flutter.dart';
import 'package:path_provider/path_provider.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {

  @override
  void initState() {
    super.initState();
    initEZLog();
  }

  Future<void> initEZLog() async {
    EZLog.init(true);
    Directory appDocDir = await getApplicationSupportDirectory();
    String logDir = '${appDocDir.path}/ezlog';

    var logger = EZLogger.config(
        EZLogConfig.plaintext("main", Level.trace.id, logDir, 7));
    
    logger.d("init", "success");

    var logs = await EZLog.requestLogFilesForDate("main", "2022_08_25");
  }
}
```