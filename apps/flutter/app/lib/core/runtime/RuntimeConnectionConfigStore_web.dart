// ignore_for_file: file_names

import 'dart:convert';

import 'package:web/web.dart' as web;

import 'RuntimeConnectionManager.dart';

const String _localRuntimeStorageKey = 'operit2.client.runtime.local_storage';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  /// Reads the browser runtime storage selection before the runtime is created.
  static Future<RuntimeConnectionConfig> read() async {
    final encoded = web.window.localStorage.getItem(_localRuntimeStorageKey);
    if (encoded != null) {
      final localStorage = LocalRuntimeStorageConfig.fromJson(
        jsonDecode(encoded) as Map<String, Object?>,
      );
      return RuntimeConnectionConfig(
        localStorage: localStorage,
        updatedAt: localStorage.updatedAt,
      );
    }
    return RuntimeConnectionConfig.local();
  }

  /// Persists the browser runtime storage selection before the runtime is created.
  static Future<void> write(RuntimeConnectionConfig config) async {
    web.window.localStorage.setItem(
      _localRuntimeStorageKey,
      jsonEncode(config.localStorage.toJson()),
    );
  }
}
