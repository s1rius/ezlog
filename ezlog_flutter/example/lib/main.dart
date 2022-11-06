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
  String _initState = 'Unknown';
  String _logFiles = 'None';

  @override
  void initState() {
    super.initState();
    initEZLog();
  }

  Future<void> initEZLog() async {
    EZLog.init(true);
    Directory appDocDir = await getApplicationSupportDirectory();
    String logDir = '${appDocDir.path}/ezlog';

    var logger = EZLogger.createLog(
        EZLogConfig.plaintext("main", Level.trace.id, logDir, 7));
    logger.d("init", "success");

    var logs = await EZLog.requestLogFilesForDate("main", "2022_08_25");
    _initState = "init ok";
    if (logs != null) {
      setState(() {
        _logFiles = logs.join(",\n");
      });
    }
    EZLog.trim();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(
          title: const Text('Plugin EZLog Demo'),
        ),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(20.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.center,
              children: <Widget>[
                const FlutterLogo(
                  size: 60,
                ),
                const SizedBox(width: 0, height: 20),
                Text("log state: $_initState\n"),
                Text(_logFiles)
              ],
            ),
          ),
        ),
      ),
    );
  }
}
