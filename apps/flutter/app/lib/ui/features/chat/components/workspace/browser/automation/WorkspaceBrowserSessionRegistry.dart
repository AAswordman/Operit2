// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/scheduler.dart';

import 'WorkspaceBrowserAutomationController.dart';

class WorkspaceBrowserSessionInfo {
  const WorkspaceBrowserSessionInfo({
    required this.sessionId,
    required this.title,
    required this.url,
    required this.active,
  });

  final String sessionId;
  final String title;
  final String url;
  final bool active;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'sessionId': sessionId,
      'title': title,
      'url': url,
      'active': active,
    };
  }
}

class _WorkspaceBrowserControls {
  const _WorkspaceBrowserControls({required this.revealBrowserTab});

  final VoidCallback revealBrowserTab;
}

class _WorkspaceBrowserSessionOpener {
  const _WorkspaceBrowserSessionOpener({required this.openBrowserTab});

  final Future<void> Function({
    String? url,
    String? userAgent,
    Map<String, String>? headers,
  })
  openBrowserTab;
}

class _WorkspaceBrowserOpenRequest {
  const _WorkspaceBrowserOpenRequest({this.url});

  final String? url;
}

class _WorkspaceBrowserSessionControls {
  const _WorkspaceBrowserSessionControls({
    required this.selectTab,
    required this.closeTab,
    required this.navigate,
    required this.navigateBack,
  });

  final void Function(String sessionId) selectTab;
  final void Function(String sessionId) closeTab;
  final void Function(String url) navigate;
  final void Function() navigateBack;
}

class WorkspaceBrowserSessionRegistry extends ChangeNotifier {
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

  WorkspaceBrowserAutomationController? get activeController {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      return null;
    }
    return _controllers[sessionId];
  }

  List<WorkspaceBrowserSessionInfo> get sessions =>
      List<WorkspaceBrowserSessionInfo>.unmodifiable(_sessions.values);

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

  void clearBrowserControls(Object owner) {
    if (!identical(_browserControlsOwner, owner)) {
      return;
    }
    _browserControlsOwner = null;
    _browserControls = null;
  }

  bool hasBrowserControls() {
    return _browserControls != null;
  }

  List<Map<String, Object?>> listTabs() {
    return sessions.map((session) => session.toJson()).toList(growable: false);
  }

  Future<void> openBrowserTab({String? url}) async {
    final opener = _browserSessionOpener;
    if (opener == null) {
      _pendingOpenRequests.add(_WorkspaceBrowserOpenRequest(url: url));
      return;
    }
    await opener.openBrowserTab(url: url);
    revealBrowserTab();
  }

  void revealBrowserTab() {
    _browserControls?.revealBrowserTab();
  }

  Future<void> waitForSession({required Duration timeout}) {
    if (activeController != null) {
      return Future<void>.value();
    }
    final completer = Completer<void>();
    _sessionWaiters.add(completer);
    return completer.future.timeout(timeout);
  }

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

  void closeActiveTab() {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      throw StateError('No active browser session');
    }
    closeTab(sessionId);
  }

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

  void register({
    required String sessionId,
    required WorkspaceBrowserAutomationController controller,
    required String title,
    required String url,
    required bool active,
    required void Function(String sessionId) selectTab,
    required void Function(String sessionId) closeTab,
    required void Function(String url) navigate,
    required void Function() navigateBack,
  }) {
    _controllers[sessionId] = controller;
    _sessionControls[sessionId] = _WorkspaceBrowserSessionControls(
      selectTab: selectTab,
      closeTab: closeTab,
      navigate: navigate,
      navigateBack: navigateBack,
    );
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      sessionId: sessionId,
      title: title,
      url: url,
      active: false,
    );
    if (active) {
      _setActiveSession(sessionId);
    }
    _scheduleNotifyListeners();
  }

  void update({
    required String sessionId,
    required String title,
    required String url,
    required bool active,
  }) {
    if (!_controllers.containsKey(sessionId)) {
      return;
    }
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      sessionId: sessionId,
      title: title,
      url: url,
      active: false,
    );
    if (active) {
      _setActiveSession(sessionId);
    }
    _scheduleNotifyListeners();
  }

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

  void _setActiveSession(String sessionId) {
    _activeSessionId = sessionId;
    final entries = List<WorkspaceBrowserSessionInfo>.of(_sessions.values);
    for (final session in entries) {
      _sessions[session.sessionId] = WorkspaceBrowserSessionInfo(
        sessionId: session.sessionId,
        title: session.title,
        url: session.url,
        active: session.sessionId == sessionId,
      );
    }
    _completeSessionWaiters();
  }

  void _completeSessionWaiters() {
    final waiters = List<Completer<void>>.of(_sessionWaiters);
    _sessionWaiters.clear();
    for (final waiter in waiters) {
      if (!waiter.isCompleted) {
        waiter.complete();
      }
    }
  }

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

  Future<void> _openPendingRequest(
    _WorkspaceBrowserSessionOpener opener,
    _WorkspaceBrowserOpenRequest request,
  ) async {
    await opener.openBrowserTab(url: request.url);
    revealBrowserTab();
  }
}
