// ignore_for_file: file_names

import 'dart:convert';

import '../path/OperitClientPaths.dart';
import 'RuntimeConnectionManager.dart';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  static Future<RuntimeConnectionConfig> read() async {
    final file = await OperitClientPaths.runtimeConnectionConfigFile();
    if (!await file.exists()) {
      return RuntimeConnectionConfig.local();
    }
    final content = await file.readAsString();
    return RuntimeConnectionConfig.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }

  static Future<void> write(RuntimeConnectionConfig config) async {
    final file = await OperitClientPaths.runtimeConnectionConfigFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent('  ').convert(config),
    );
  }
}
