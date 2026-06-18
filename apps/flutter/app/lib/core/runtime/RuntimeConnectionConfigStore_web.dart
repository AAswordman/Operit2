// ignore_for_file: file_names

import 'dart:convert';
import 'dart:html' as html;

import '../link/RemoteRuntimeLinkClient.dart';
import 'RuntimeConnectionManager.dart';

const String _runtimeConnectionStorageKey =
    'operit2.client.link.runtime_connection';
const String _outboundSessionsStorageKey =
    'operit2.client.link.outbound_sessions';

class RuntimeConnectionConfigStore {
  const RuntimeConnectionConfigStore._();

  static Future<RuntimeConnectionConfig> read() async {
    final remoteSessions = await OutboundLinkSessionStore.read();
    final content = html.window.localStorage[_runtimeConnectionStorageKey];
    if (content == null) {
      return RuntimeConnectionConfig.local().copyWith(
        remoteSessions: remoteSessions,
      );
    }
    return RuntimeConnectionConfig.fromJson(
      jsonDecode(content) as Map<String, Object?>,
      remoteSessions: remoteSessions,
    );
  }

  static Future<void> write(RuntimeConnectionConfig config) async {
    html.window.localStorage[_runtimeConnectionStorageKey] =
        const JsonEncoder.withIndent('  ').convert(config);
  }
}

class OutboundLinkSessionStore {
  const OutboundLinkSessionStore._();

  static Future<Map<String, PairedRemoteSessionRecord>> read() async {
    final content = html.window.localStorage[_outboundSessionsStorageKey];
    if (content == null) {
      return <String, PairedRemoteSessionRecord>{};
    }
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
    html.window.localStorage[_outboundSessionsStorageKey] =
        const JsonEncoder.withIndent(
          '  ',
        ).convert(sessions.map((key, value) => MapEntry(key, value.toJson())));
  }
}
