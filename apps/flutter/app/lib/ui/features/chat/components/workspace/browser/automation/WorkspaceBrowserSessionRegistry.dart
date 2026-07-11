// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

import 'WorkspaceBrowserAutomationController.dart';

class WorkspaceBrowserSessionInfo {
  /// Creates a runtime-visible browser session snapshot.
  const WorkspaceBrowserSessionInfo({
    required this.sessionId,
    required this.title,
    required this.url,
    required this.active,
    required this.userAgent,
    required this.canGoBack,
    required this.canGoForward,
    required this.isLoading,
    required this.progress,
  });

  final String sessionId;
  final String title;
  final String url;
  final bool active;
  final String? userAgent;
  final bool canGoBack;
  final bool canGoForward;
  final bool isLoading;
  final int progress;

  /// Serializes this workspace session for local UI automation.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'title': title,
      'url': url,
      'active': active,
      'userAgent': userAgent,
      'canGoBack': canGoBack,
      'canGoForward': canGoForward,
      'isLoading': isLoading,
      'progress': progress,
    };
  }

  /// Converts this workspace session into the runtime browser model.
  RuntimeBrowserSessionInfo toRuntimeBrowserSessionInfo() {
    return RuntimeBrowserSessionInfo(
      sessionId: sessionId,
      currentUrl: url,
      title: title,
      userAgent: userAgent,
      active: active,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
      isLoading: isLoading,
      progress: progress,
    );
  }
}

class _WorkspaceBrowserControls {
  /// Creates workspace-level browser chrome controls.
  const _WorkspaceBrowserControls({required this.revealBrowserTab});

  final VoidCallback revealBrowserTab;
}

class _WorkspaceBrowserSessionOpener {
  /// Creates the owner-host browser tab opener.
  const _WorkspaceBrowserSessionOpener({required this.openBrowserTab});

  final Future<void> Function({
    String? url,
    String? userAgent,
    Map<String, String>? headers,
  })
  openBrowserTab;
}

class _WorkspaceBrowserOpenRequest {
  /// Captures a browser open request until the owner host attaches.
  const _WorkspaceBrowserOpenRequest({this.url, this.userAgent, this.headers});

  final String? url;
  final String? userAgent;
  final Map<String, String>? headers;
}

class _WorkspaceBrowserSessionControls {
  /// Creates host controls for a concrete browser session.
  const _WorkspaceBrowserSessionControls({
    required this.selectTab,
    required this.closeTab,
    required this.navigate,
    required this.navigateBack,
    required this.navigateForward,
    required this.reload,
    required this.stop,
    required this.supportsPageJavaScript,
  });

  final void Function(String sessionId) selectTab;
  final void Function(String sessionId) closeTab;
  final void Function(String url) navigate;
  final void Function() navigateBack;
  final void Function() navigateForward;
  final void Function() reload;
  final void Function() stop;
  final bool Function() supportsPageJavaScript;
}

class WorkspaceBrowserSessionRegistry extends ChangeNotifier {
  /// Creates the singleton browser owner-host registry.
  WorkspaceBrowserSessionRegistry._();

  static final WorkspaceBrowserSessionRegistry instance =
      WorkspaceBrowserSessionRegistry._();

  final Map<String, WorkspaceBrowserAutomationController> _controllers =
      <String, WorkspaceBrowserAutomationController>{};
  final Map<String, WorkspaceBrowserSessionInfo> _sessions =
      <String, WorkspaceBrowserSessionInfo>{};
  _WorkspaceBrowserControls? _browserControls;
  Object? _browserControlsOwner;
  _WorkspaceBrowserSessionOpener? _browserSessionOpener;
  final Map<String, _WorkspaceBrowserSessionControls> _sessionControls =
      <String, _WorkspaceBrowserSessionControls>{};
  final List<Completer<void>> _sessionWaiters = <Completer<void>>[];
  final List<_WorkspaceBrowserOpenRequest> _pendingOpenRequests =
      <_WorkspaceBrowserOpenRequest>[];
  String? _activeSessionId;
  bool _notifyScheduled = false;

  String? get activeSessionId => _activeSessionId;

  /// Returns the active browser automation controller.
  WorkspaceBrowserAutomationController? get activeController {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      return null;
    }
    return _controllers[sessionId];
  }

  List<WorkspaceBrowserSessionInfo> get sessions =>
      List<WorkspaceBrowserSessionInfo>.unmodifiable(_sessions.values);

  /// Registers workspace-level browser controls owned by the visible UI.
  void setBrowserControls({
    required Object owner,
    required VoidCallback revealBrowserTab,
  }) {
    final controls = _WorkspaceBrowserControls(
      revealBrowserTab: revealBrowserTab,
    );
    _browserControlsOwner = owner;
    _browserControls = controls;
    _drainPendingOpenRequests();
  }

  /// Registers the host callback that creates real browser tabs.
  void setBrowserSessionOpener({
    required Future<void> Function({
      String? url,
      String? userAgent,
      Map<String, String>? headers,
    })
    openBrowserTab,
  }) {
    _browserSessionOpener = _WorkspaceBrowserSessionOpener(
      openBrowserTab: openBrowserTab,
    );
    _drainPendingOpenRequests();
  }

  /// Clears browser controls when their owning UI is detached.
  void clearBrowserControls(Object owner) {
    if (!identical(_browserControlsOwner, owner)) {
      return;
    }
    _browserControlsOwner = null;
    _browserControls = null;
  }

  /// Returns whether visible browser controls are currently attached.
  bool hasBrowserControls() {
    return _browserControls != null;
  }

  /// Lists browser tabs for local automation surfaces.
  List<Map<String, Object?>> listTabs() {
    return sessions.map((session) => session.toJson()).toList(growable: false);
  }

  /// Opens a browser tab through the registered host opener.
  Future<void> openBrowserTab({
    String? url,
    String? userAgent,
    Map<String, String>? headers,
  }) async {
    final opener = _browserSessionOpener;
    if (opener == null) {
      _pendingOpenRequests.add(
        _WorkspaceBrowserOpenRequest(
          url: url,
          userAgent: userAgent,
          headers: headers,
        ),
      );
      return;
    }
    await opener.openBrowserTab(
      url: url,
      userAgent: userAgent,
      headers: headers,
    );
    revealBrowserTab();
  }

  /// Reveals the visible browser workspace.
  void revealBrowserTab() {
    _browserControls?.revealBrowserTab();
  }

  /// Waits until at least one browser session is registered.
  Future<void> waitForSession({required Duration timeout}) {
    if (activeController != null) {
      return Future<void>.value();
    }
    final completer = Completer<void>();
    _sessionWaiters.add(completer);
    return completer.future.timeout(timeout);
  }

  /// Selects a registered browser tab.
  void selectTab(String sessionId) {
    final session = _sessions[sessionId];
    if (session == null) {
      throw StateError('Browser session is not registered');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.selectTab(sessionId);
    revealBrowserTab();
  }

  /// Closes a registered browser tab.
  void closeTab(String sessionId) {
    final session = _sessions[sessionId];
    if (session == null) {
      throw StateError('Browser session is not registered');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.closeTab(sessionId);
  }

  /// Closes the active browser tab.
  void closeActiveTab() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    closeTab(sessionId);
  }

  /// Closes every registered browser tab.
  void closeAllTabs() {
    final sessionIds = _sessions.values
        .map((session) => session.sessionId)
        .toList(growable: false);
    for (final sessionId in sessionIds) {
      if (_sessions.containsKey(sessionId)) {
        closeTab(sessionId);
      }
    }
  }

  /// Navigates the active browser tab.
  void navigate(String url) {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.navigate(url);
  }

  /// Navigates the active browser tab backward.
  void navigateBack() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.navigateBack();
  }

  /// Navigates the active browser tab forward.
  void navigateForward() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.navigateForward();
  }

  /// Reloads the active browser tab.
  void reload() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.reload();
  }

  /// Stops loading the active browser tab.
  void stop() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.stop();
  }

  /// Executes a runtime browser command on the owner host.
  Future<RuntimeBrowserCommandResult> handleRuntimeBrowserCommand(
    RuntimeBrowserCommand command,
  ) async {
    try {
      return await _handleRuntimeBrowserCommand(command);
    } catch (error) {
      return RuntimeBrowserCommandResult(
        success: false,
        session: _sessionForCommand(command)?.toRuntimeBrowserSessionInfo(),
        sessions: runtimeSessions,
        resultJson: '',
        error: error.toString(),
      );
    }
  }

  /// Executes a serialized runtime browser command on the owner host.
  Future<String> handleRuntimeBrowserCommandJson(String commandJson) async {
    final command = RuntimeBrowserCommand.fromJson(
      jsonDecode(commandJson) as Map<String, Object?>,
    );
    final result = await handleRuntimeBrowserCommand(command);
    return jsonEncode(result.toJson());
  }

  /// Returns the current browser sessions in runtime model form.
  List<RuntimeBrowserSessionInfo> get runtimeSessions {
    return _sessions.values
        .map((session) => session.toRuntimeBrowserSessionInfo())
        .toList(growable: false);
  }

  /// Registers a browser session and its host controls.
  void register({
    required String sessionId,
    required WorkspaceBrowserAutomationController controller,
    required String title,
    required String url,
    required bool active,
    required String? userAgent,
    required bool canGoBack,
    required bool canGoForward,
    required bool isLoading,
    required int progress,
    required void Function(String sessionId) selectTab,
    required void Function(String sessionId) closeTab,
    required void Function(String url) navigate,
    required void Function() navigateBack,
    required void Function() navigateForward,
    required void Function() reload,
    required void Function() stop,
    required bool Function() supportsPageJavaScript,
  }) {
    _controllers[sessionId] = controller;
    _sessionControls[sessionId] = _WorkspaceBrowserSessionControls(
      selectTab: selectTab,
      closeTab: closeTab,
      navigate: navigate,
      navigateBack: navigateBack,
      navigateForward: navigateForward,
      reload: reload,
      stop: stop,
      supportsPageJavaScript: supportsPageJavaScript,
    );
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      sessionId: sessionId,
      title: title,
      url: url,
      active: false,
      userAgent: userAgent,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
      isLoading: isLoading,
      progress: progress,
    );
    if (active) {
      _setActiveSession(sessionId);
    }
    _scheduleNotifyListeners();
  }

  /// Updates a registered browser session snapshot.
  void update({
    required String sessionId,
    required String title,
    required String url,
    required bool active,
    required String? userAgent,
    required bool canGoBack,
    required bool canGoForward,
    required bool isLoading,
    required int progress,
  }) {
    if (!_controllers.containsKey(sessionId)) {
      return;
    }
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      sessionId: sessionId,
      title: title,
      url: url,
      active: false,
      userAgent: userAgent,
      canGoBack: canGoBack,
      canGoForward: canGoForward,
      isLoading: isLoading,
      progress: progress,
    );
    if (active) {
      _setActiveSession(sessionId);
    }
    _scheduleNotifyListeners();
  }

  /// Removes a browser session from the owner registry.
  void unregister(String sessionId) {
    _controllers.remove(sessionId);
    _sessionControls.remove(sessionId);
    _sessions.remove(sessionId);
    if (_activeSessionId == sessionId) {
      final nextSessionId = _sessions.isEmpty ? null : _sessions.keys.last;
      _activeSessionId = null;
      if (nextSessionId != null) {
        _setActiveSession(nextSessionId);
      }
    }
    _scheduleNotifyListeners();
  }

  /// Executes a runtime browser command and returns its result.
  Future<RuntimeBrowserCommandResult> _handleRuntimeBrowserCommand(
    RuntimeBrowserCommand command,
  ) async {
    switch (command.action) {
      case 'list':
        return _commandResult(success: true, command: command);
      case 'create':
        return await _createRuntimeSession(command);
      case 'update':
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'navigate':
        _selectSessionForCommand(command);
        _controlsForCommand(command).navigate(_requireCommandUrl(command));
        if (command.reveal) {
          revealBrowserTab();
        }
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'back':
        _selectSessionForCommand(command);
        _controlsForCommand(command).navigateBack();
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'forward':
        _selectSessionForCommand(command);
        _controlsForCommand(command).navigateForward();
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'reload':
        _selectSessionForCommand(command);
        _controlsForCommand(command).reload();
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'stop':
        _selectSessionForCommand(command);
        _controlsForCommand(command).stop();
        return _commandResult(
          success: true,
          command: command,
          session: _requireSession(command),
        );
      case 'close':
        final session = _requireSession(command);
        _controlsForCommand(command).closeTab(session.sessionId);
        return _commandResult(
          success: true,
          command: command,
          session: session,
        );
      case 'snapshot':
        return await _snapshotRuntimeSession(command);
      case 'evaluate':
        return await _evaluateRuntimeSession(command);
      default:
        throw StateError('Unknown browser command action: ${command.action}');
    }
  }

  /// Creates a browser session through the owner host tab opener.
  Future<RuntimeBrowserCommandResult> _createRuntimeSession(
    RuntimeBrowserCommand command,
  ) async {
    final before = _sessions.keys.toSet();
    await openBrowserTab(
      url: _requireCommandUrl(command),
      userAgent: command.userAgent,
      headers: command.headers,
    );
    await waitForSession(timeout: const Duration(seconds: 20));
    final openedSession = _latestOpenedSession(before);
    final session = openedSession ?? _requireActiveSession();
    return _commandResult(success: true, command: command, session: session);
  }

  /// Captures the current host-side browser session snapshot.
  Future<RuntimeBrowserCommandResult> _snapshotRuntimeSession(
    RuntimeBrowserCommand command,
  ) async {
    final session = _requireSession(command);
    final controls = _controlsForCommand(command);
    if (!controls.supportsPageJavaScript()) {
      throw StateError(
        'Browser session does not expose page JavaScript to the runtime host',
      );
    }
    final controller = _controllers[session.sessionId];
    if (controller == null) {
      throw StateError('Browser session controller is not registered');
    }
    final result = await controller.snapshot();
    return _commandResult(
      success: true,
      command: command,
      session: session,
      resultJson: _runtimeResultJson(result),
    );
  }

  /// Evaluates JavaScript in a host-owned browser session.
  Future<RuntimeBrowserCommandResult> _evaluateRuntimeSession(
    RuntimeBrowserCommand command,
  ) async {
    final session = _requireSession(command);
    final controls = _controlsForCommand(command);
    if (!controls.supportsPageJavaScript()) {
      throw StateError(
        'Browser session does not expose page JavaScript to the runtime host',
      );
    }
    final script = command.script;
    if (script == null || script.trim().isEmpty) {
      throw StateError('Browser evaluate command is missing script');
    }
    final controller = _controllers[session.sessionId];
    if (controller == null) {
      throw StateError('Browser session controller is not registered');
    }
    final result = await controller.runCode(script);
    return _commandResult(
      success: true,
      command: command,
      session: session,
      resultJson: _runtimeResultJson(result),
    );
  }

  /// Builds a runtime command result.
  RuntimeBrowserCommandResult _commandResult({
    required bool success,
    required RuntimeBrowserCommand command,
    WorkspaceBrowserSessionInfo? session,
    String resultJson = '',
  }) {
    return RuntimeBrowserCommandResult(
      success: success,
      session: session?.toRuntimeBrowserSessionInfo(),
      sessions: runtimeSessions,
      resultJson: resultJson,
      error: null,
    );
  }

  /// Returns the session targeted by the command.
  WorkspaceBrowserSessionInfo? _sessionForCommand(
    RuntimeBrowserCommand command,
  ) {
    final sessionId = command.sessionId;
    if (sessionId != null) {
      return _sessions[sessionId];
    }
    final activeSessionId = _activeSessionId;
    if (activeSessionId == null) {
      return null;
    }
    return _sessions[activeSessionId];
  }

  /// Returns the required session targeted by the command.
  WorkspaceBrowserSessionInfo _requireSession(RuntimeBrowserCommand command) {
    final session = _sessionForCommand(command);
    if (session == null) {
      throw StateError('Browser session is not registered');
    }
    return session;
  }

  /// Returns the required active session.
  WorkspaceBrowserSessionInfo _requireActiveSession() {
    final activeSessionId = _activeSessionId;
    if (activeSessionId == null) {
      throw StateError('No active browser session');
    }
    final session = _sessions[activeSessionId];
    if (session == null) {
      throw StateError('Active browser session is not registered');
    }
    return session;
  }

  /// Returns the controls targeted by the command.
  _WorkspaceBrowserSessionControls _controlsForCommand(
    RuntimeBrowserCommand command,
  ) {
    final session = _requireSession(command);
    final controls = _sessionControls[session.sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    return controls;
  }

  /// Selects the browser session targeted by the command.
  void _selectSessionForCommand(RuntimeBrowserCommand command) {
    final session = _requireSession(command);
    final controls = _sessionControls[session.sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.selectTab(session.sessionId);
    if (command.reveal) {
      revealBrowserTab();
    }
  }

  /// Returns the URL required by a navigation command.
  String _requireCommandUrl(RuntimeBrowserCommand command) {
    final url = command.url?.trim();
    if (url == null || url.isEmpty) {
      throw StateError('Browser command is missing url');
    }
    return url;
  }

  /// Finds the session opened after a create command began.
  WorkspaceBrowserSessionInfo? _latestOpenedSession(Set<String> before) {
    final opened = _sessions.values
        .where((session) => !before.contains(session.sessionId))
        .toList(growable: false);
    if (opened.isEmpty) {
      return null;
    }
    return opened.last;
  }

  /// Serializes a WebView JavaScript result for the runtime command result.
  String _runtimeResultJson(Object? result) {
    if (result == null) {
      return 'null';
    }
    if (result is String) {
      return jsonEncode(result);
    }
    return jsonEncode(result);
  }

  /// Marks one browser session as active.
  void _setActiveSession(String sessionId) {
    _activeSessionId = sessionId;
    final entries = List<WorkspaceBrowserSessionInfo>.of(_sessions.values);
    for (final session in entries) {
      _sessions[session.sessionId] = WorkspaceBrowserSessionInfo(
        sessionId: session.sessionId,
        title: session.title,
        url: session.url,
        active: session.sessionId == sessionId,
        userAgent: session.userAgent,
        canGoBack: session.canGoBack,
        canGoForward: session.canGoForward,
        isLoading: session.isLoading,
        progress: session.progress,
      );
    }
    _completeSessionWaiters();
  }

  /// Completes pending waiters for newly available browser sessions.
  void _completeSessionWaiters() {
    final waiters = List<Completer<void>>.of(_sessionWaiters);
    _sessionWaiters.clear();
    for (final waiter in waiters) {
      if (!waiter.isCompleted) {
        waiter.complete();
      }
    }
  }

  /// Schedules registry listener notification after the current frame.
  void _scheduleNotifyListeners() {
    if (_notifyScheduled) {
      return;
    }
    _notifyScheduled = true;
    SchedulerBinding.instance.addPostFrameCallback((_) {
      _notifyScheduled = false;
      notifyListeners();
    });
    SchedulerBinding.instance.ensureVisualUpdate();
  }

  /// Opens browser tabs that were requested before the host opener attached.
  void _drainPendingOpenRequests() {
    final opener = _browserSessionOpener;
    if (opener == null) {
      return;
    }
    final requests = List<_WorkspaceBrowserOpenRequest>.of(
      _pendingOpenRequests,
    );
    _pendingOpenRequests.clear();
    for (final request in requests) {
      unawaited(_openPendingRequest(opener, request));
    }
  }

  /// Opens one queued browser tab request.
  Future<void> _openPendingRequest(
    _WorkspaceBrowserSessionOpener opener,
    _WorkspaceBrowserOpenRequest request,
  ) async {
    await opener.openBrowserTab(
      url: request.url,
      userAgent: request.userAgent,
      headers: request.headers,
    );
    revealBrowserTab();
  }
}
