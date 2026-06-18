// ignore_for_file: file_names

import 'dart:convert';

import '../link/RemoteRuntimeLinkClient.dart';
import '../path/OperitClientPaths.dart';
import 'RuntimeConnectionManager.dart';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  static Future<RuntimeConnectionConfig> read() async {
    final remoteSessions = await OutboundLinkSessionStore.read();
    final file = await OperitClientPaths.runtimeConnectionConfigFile();
    if (!await file.exists()) {
      return RuntimeConnectionConfig.local().copyWith(
        remoteSessions: remoteSessions,
      );
    }
    final content = await file.readAsString();
    return RuntimeConnectionConfig.fromJson(
      jsonDecode(content) as Map<String, Object?>,
      remoteSessions: remoteSessions,
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

class OutboundLinkSessionStore {
  const OutboundLinkSessionStore._();

  static Future<Map<String, PairedRemoteSessionRecord>> read() async {
    final file = await OperitClientPaths.outboundLinkSessionsFile();
    if (!await file.exists()) {
      return <String, PairedRemoteSessionRecord>{};
    }
    final content = await file.readAsString();
    final decoded = jsonDecode(content) as Map<String, Object?>;
    return decoded.map(
      (key, value) => MapEntry(
        key,
        PairedRemoteSessionRecord.fromJson(value as Map<String, Object?>),
      ),
    );
  }

  static Future<void> write(
    Map<String, PairedRemoteSessionRecord> sessions,
  ) async {
    final file = await OperitClientPaths.outboundLinkSessionsFile();
    await file.parent.create(recursive: true);
    await file.writeAsString(
      const JsonEncoder.withIndent(
        '  ',
      ).convert(sessions.map((key, value) => MapEntry(key, value.toJson()))),
    );
  }
}
