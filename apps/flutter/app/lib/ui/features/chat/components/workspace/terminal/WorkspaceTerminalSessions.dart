// ignore_for_file: file_names

import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;

class WorkspaceTerminalSessionInfo {
  const WorkspaceTerminalSessionInfo({
    required this.sessionId,
    required this.sessionName,
    required this.terminalType,
    required this.sessionKind,
    required this.workingDir,
    required this.commandRunning,
  });

  factory WorkspaceTerminalSessionInfo.fromCore(
    core_proxy.RuntimeTerminalSessionInfo info,
  ) {
    return WorkspaceTerminalSessionInfo(
      sessionId: info.sessionId,
      sessionName: info.sessionName,
      terminalType: info.terminalType,
      sessionKind: info.sessionKind,
      workingDir: info.workingDir,
      commandRunning: info.commandRunning,
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

  factory WorkspaceTerminalScreen.fromCore(
    core_proxy.RuntimeTerminalScreen screen,
  ) {
    return WorkspaceTerminalScreen(
      sessionId: screen.sessionId,
      terminalType: screen.terminalType,
      rows: screen.rows,
      cols: screen.cols,
      content: screen.content,
      commandRunning: screen.commandRunning,
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
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  final GeneratedCoreProxyClients _clients;

  GeneratedServicesRuntimeTerminalServiceCoreProxy get _terminal =>
      _clients.servicesRuntimeTerminalService;

  Future<List<WorkspaceTerminalSessionInfo>> listSessions() async {
    final sessions = await _terminal.terminalSessionsFlowSnapshot();
    return sessions
        .map(WorkspaceTerminalSessionInfo.fromCore)
        .toList(growable: false);
  }

  Stream<List<WorkspaceTerminalSessionInfo>> watchSessions() {
    return _terminal.terminalSessionsFlowChanges().map(
      (sessions) => sessions
          .map(WorkspaceTerminalSessionInfo.fromCore)
          .toList(growable: false),
    );
  }

  /// Returns the host-declared terminal type for manual PTY creation.
  Future<String> defaultTerminalType() {
    return _terminal.defaultTerminalType();
  }

  /// Returns every terminal type exposed by the active runtime host.
  Future<core_proxy.RuntimeTerminalInfo> terminalInfo() {
    return _terminal.terminalInfo();
  }

  /// Starts a typed PTY session.
  Future<String> startPtySession({
    required String sessionName,
    required String terminalType,
    required String workingDirectory,
    required int rows,
    required int columns,
  }) {
    return _terminal.startTerminalPty(
      sessionName: sessionName,
      terminalType: terminalType,
      workingDir: workingDirectory,
      rows: rows,
      cols: columns,
    );
  }

  Future<WorkspaceTerminalScreen> getSessionScreen(String sessionId) async {
    final screen = await _terminal.getTerminalSessionScreen(
      sessionId: sessionId,
    );
    return WorkspaceTerminalScreen.fromCore(screen);
  }

  Future<void> inputSession({
    required String sessionId,
    required String input,
  }) async {
    await _terminal.inputTerminalSession(sessionId: sessionId, input: input);
  }

  Future<void> closePtySession(String sessionId) {
    return _terminal.closeTerminalPty(sessionId: sessionId);
  }
}
