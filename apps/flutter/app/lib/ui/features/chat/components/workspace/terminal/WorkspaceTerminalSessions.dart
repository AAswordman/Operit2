// ignore_for_file: file_names

import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;

/// Uses the generated Core terminal session state directly in Flutter.
typedef WorkspaceTerminalSessionInfo =
    core_proxy.RuntimeTerminalSessionInfo;

/// Adds the terminal title used only by the Flutter presentation layer.
extension WorkspaceTerminalSessionInfoPresentation
    on core_proxy.RuntimeTerminalSessionInfo {
  /// Returns the trimmed title displayed for this terminal session.
  String get title => sessionName.trim();
}

/// Uses the generated Core terminal screen directly in Flutter.
typedef WorkspaceTerminalScreen = core_proxy.RuntimeTerminalScreen;

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
    return _terminal.terminalSessionsFlow().first;
  }

  Stream<List<WorkspaceTerminalSessionInfo>> watchSessions() {
    return _terminal.terminalSessionsFlow();
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
    required String terminal,
    required String terminalType,
    required String workingDirectory,
    required int rows,
    required int columns,
  }) {
    return _terminal.startTerminalPty(
      sessionName: sessionName,
      terminal: terminal,
      terminalType: terminalType,
      workingDir: workingDirectory,
      rows: rows,
      cols: columns,
    );
  }

  Future<WorkspaceTerminalScreen> getSessionScreen(String sessionId) async {
    return _terminal.getTerminalSessionScreen(
      sessionId: sessionId,
    );
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
