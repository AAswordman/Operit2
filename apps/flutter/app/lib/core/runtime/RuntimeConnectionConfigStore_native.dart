// ignore_for_file: file_names

import 'dart:convert';
import 'dart:io';

import 'package:path/path.dart' as path;
import 'package:path_provider/path_provider.dart';

import 'RuntimeConnectionManager.dart';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  /// Reads the local runtime storage selection before the runtime is created.
  static Future<RuntimeConnectionConfig> read() async {
    final file = await _configFile();
    if (await file.exists()) {
      final decoded = jsonDecode(await file.readAsString()) as Map<String, Object?>;
      final localStorage = LocalRuntimeStorageConfig.fromJson(decoded);
      return RuntimeConnectionConfig(
        localStorage: localStorage,
        updatedAt: localStorage.updatedAt,
      );
    }
    return RuntimeConnectionConfig.local();
  }

  /// Persists the local runtime storage selection before the runtime is created.
  static Future<void> write(RuntimeConnectionConfig config) async {
    final file = await _configFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(jsonEncode(config.localStorage.toJson()));
  }

  /// Resolves the bootstrap configuration file outside the runtime storage root.
  static Future<File> _configFile() async {
    final supportDirectory = await getApplicationSupportDirectory();
    return File(
      path.join(
        supportDirectory.path,
        'client',
        'link',
        'local_runtime_storage.json',
      ),
    );
  }
}
