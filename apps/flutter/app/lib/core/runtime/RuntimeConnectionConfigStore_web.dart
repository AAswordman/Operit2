// ignore_for_file: file_names

import 'dart:convert';
import 'dart:html' as html;

import 'RuntimeConnectionManager.dart';

const String _runtimeConnectionStorageKey = 'operit2.client.runtime_connection';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  static Future<RuntimeConnectionConfig> read() async {
    final content = html.window.localStorage[_runtimeConnectionStorageKey];
    if (content == null) {
      return RuntimeConnectionConfig.local();
    }
    return RuntimeConnectionConfig.fromJson(
      jsonDecode(content) as Map<String, Object?>,
    );
  }

  static Future<void> write(RuntimeConnectionConfig config) async {
    html.window.localStorage[_runtimeConnectionStorageKey] =
        const JsonEncoder.withIndent('  ').convert(config);
  }
}
