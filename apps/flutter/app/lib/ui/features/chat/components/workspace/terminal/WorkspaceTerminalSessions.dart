// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

class WorkspaceTerminalSessionInfo {
  const WorkspaceTerminalSessionInfo({
    required this.sessionId,
    required this.sessionName,
    required this.terminalType,
    required this.sessionKind,
    required this.workingDir,
    required this.commandRunning,
  });

  factory WorkspaceTerminalSessionInfo.fromJson(Map<String, dynamic> json) {
    return WorkspaceTerminalSessionInfo(
      sessionId: _requiredString(json, 'sessionId'),
      sessionName: _requiredString(json, 'sessionName'),
      terminalType: _requiredString(json, 'terminalType'),
      sessionKind: _requiredString(json, 'sessionKind'),
      workingDir: _requiredString(json, 'workingDir'),
      commandRunning: json['commandRunning'] == true,
    );
  }

  final String sessionId;
  final String sessionName;
  final String terminalType;
  final String sessionKind;
  final String workingDir;
  final bool commandRunning;

  String get title {
    return sessionName.trim();
  }
}

class WorkspaceTerminalScreen {
  const WorkspaceTerminalScreen({
    required this.sessionId,
    required this.terminalType,
    required this.rows,
    required this.cols,
    required this.content,
    required this.commandRunning,
  });

  factory WorkspaceTerminalScreen.fromJson(Map<String, dynamic> json) {
    return WorkspaceTerminalScreen(
      sessionId: _requiredString(json, 'sessionId'),
      terminalType: _requiredString(json, 'terminalType'),
      rows: _requiredInt(json, 'rows'),
      cols: _requiredInt(json, 'cols'),
      content: _requiredString(json, 'content'),
      commandRunning: json['commandRunning'] == true,
    );
  }

  final String sessionId;
  final String terminalType;
  final int rows;
  final int cols;
  final String content;
  final bool commandRunning;
}

class WorkspaceTerminalSessions {
  const WorkspaceTerminalSessions({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  Future<List<WorkspaceTerminalSessionInfo>> listSessions() async {
    final response = await _invokeJson('listTerminalSessions');
    final sessions = response['sessions'];
    if (sessions is! List) {
      throw StateError('listTerminalSessions missing sessions');
    }
    return sessions
        .map((item) {
          if (item is! Map<String, dynamic>) {
            throw StateError('terminal session item is not an object');
          }
          return WorkspaceTerminalSessionInfo.fromJson(item);
        })
        .toList(growable: false);
  }

  Future<String> startPtySession({
    required String sessionName,
    required String workingDirectory,
    required int rows,
    required int columns,
  }) async {
    final response = await _invokeJson('startTerminalPty', <String, Object>{
      'sessionName': sessionName,
      'workingDirectory': workingDirectory,
      'rows': rows,
      'columns': columns,
    });
    final sessionId = response['sessionId'];
    if (sessionId is! String || sessionId.isEmpty) {
      throw StateError('startTerminalPty missing sessionId');
    }
    return sessionId;
  }

  Future<WorkspaceTerminalScreen> getSessionScreen(String sessionId) async {
    final response = await _invokeJson('getTerminalSessionScreen', sessionId);
    return WorkspaceTerminalScreen.fromJson(response);
  }

  Future<void> inputSession({
    required String sessionId,
    required String input,
  }) async {
    await _invokeJson('inputTerminalSession', <String, Object>{
      'sessionId': sessionId,
      'input': input,
    });
  }

  Future<void> closePtySession(String sessionId) async {
    await _invokeJson('closeTerminalPty', sessionId);
  }

  Future<Map<String, dynamic>> _invokeJson(
    String method, [
    Object? arguments,
  ]) async {
    final raw = await _channel.invokeMethod<String>(method, arguments);
    if (raw == null) {
      throw StateError('$method returned null');
    }
    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, dynamic>) {
      throw StateError('$method returned non-object JSON');
    }
    if (decoded['ok'] != true) {
      throw StateError(decoded['error']?.toString() ?? '$method failed');
    }
    return decoded;
  }
}

String _requiredString(Map<String, dynamic> json, String key) {
  final value = json[key];
  if (value is! String) {
    throw StateError('terminal session field $key is not a string');
  }
  return value;
}

int _requiredInt(Map<String, dynamic> json, String key) {
  final value = json[key];
  if (value is int) {
    return value;
  }
  throw StateError('terminal screen field $key is not an int');
}
