// ignore_for_file: file_names

import 'dart:async';

import 'WorkspaceBrowserAutomationController.dart';

class WorkspaceBrowserSessionInfo {
  const WorkspaceBrowserSessionInfo({
    required this.chatId,
    required this.sessionId,
    required this.title,
    required this.url,
    required this.active,
  });

  final String chatId;
  final String sessionId;
  final String title;
  final String url;
  final bool active;

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'chatId': chatId,
      'sessionId': sessionId,
      'title': title,
      'url': url,
      'active': active,
    };
  }
}

class _WorkspaceBrowserChatControls {
  const _WorkspaceBrowserChatControls({required this.openBrowserTab});

  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  openBrowserTab;
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

class WorkspaceBrowserSessionRegistry {
  WorkspaceBrowserSessionRegistry._();

  static final WorkspaceBrowserSessionRegistry instance =
      WorkspaceBrowserSessionRegistry._();

  final Map<String, WorkspaceBrowserAutomationController> _controllers =
      <String, WorkspaceBrowserAutomationController>{};
  final Map<String, WorkspaceBrowserSessionInfo> _sessions =
      <String, WorkspaceBrowserSessionInfo>{};
  final Map<String, _WorkspaceBrowserChatControls> _chatControls =
      <String, _WorkspaceBrowserChatControls>{};
  final Map<String, _WorkspaceBrowserSessionControls> _sessionControls =
      <String, _WorkspaceBrowserSessionControls>{};
  final Map<String, String> _activeSessionIdByChatId = <String, String>{};
  final Map<String, List<Completer<void>>> _chatSessionWaiters =
      <String, List<Completer<void>>>{};
  String? _activeSessionId;

  String? get activeSessionId => _activeSessionId;

  WorkspaceBrowserAutomationController? get activeController {
    final sessionId = _activeSessionId;
    if (sessionId == null) {
      return null;
    }
    return _controllers[sessionId];
  }

  WorkspaceBrowserAutomationController? activeControllerForChat(String chatId) {
    final sessionId = _activeSessionIdByChatId[chatId];
    if (sessionId == null) {
      return null;
    }
    return _controllers[sessionId];
  }

  List<WorkspaceBrowserSessionInfo> get sessions =>
      List<WorkspaceBrowserSessionInfo>.unmodifiable(_sessions.values);

  void setChatControls({
    required String chatId,
    required void Function({
      String? url,
      String? localFilePath,
      String? workspaceHtmlPath,
    })
    openBrowserTab,
  }) {
    _chatControls[chatId] = _WorkspaceBrowserChatControls(
      openBrowserTab: openBrowserTab,
    );
  }

  void clearChatControls(String chatId) {
    _chatControls.remove(chatId);
  }

  bool hasChatControls(String chatId) {
    return _chatControls.containsKey(chatId);
  }

  List<Map<String, Object?>> listTabs(String chatId) {
    return sessions
        .where((session) => session.chatId == chatId)
        .map((session) => session.toJson())
        .toList(growable: false);
  }

  void openBrowserTab(String chatId, {String? url}) {
    final controls = _chatControls[chatId];
    if (controls == null) {
      throw StateError('No browser panel registered for chat_id $chatId');
    }
    controls.openBrowserTab(url: url);
  }

  Future<void> waitForChatSession(String chatId, {required Duration timeout}) {
    if (activeControllerForChat(chatId) != null) {
      return Future<void>.value();
    }
    final completer = Completer<void>();
    _chatSessionWaiters
        .putIfAbsent(chatId, () => <Completer<void>>[])
        .add(completer);
    return completer.future.timeout(timeout);
  }

  void selectTab(String chatId, String sessionId) {
    final session = _sessions[sessionId];
    if (session == null || session.chatId != chatId) {
      throw StateError('Browser session does not belong to chat_id $chatId');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.selectTab(sessionId);
  }

  void closeTab(String chatId, String sessionId) {
    final session = _sessions[sessionId];
    if (session == null || session.chatId != chatId) {
      throw StateError('Browser session does not belong to chat_id $chatId');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.closeTab(sessionId);
  }

  void closeActiveTab(String chatId) {
    final sessionId = _activeSessionIdByChatId[chatId];
    if (sessionId == null) {
      throw StateError('No active browser session for chat_id $chatId');
    }
    closeTab(chatId, sessionId);
  }

  void closeAllTabs(String chatId) {
    final sessionIds = _sessions.values
        .where((session) => session.chatId == chatId)
        .map((session) => session.sessionId)
        .toList(growable: false);
    for (final sessionId in sessionIds) {
      if (_sessions.containsKey(sessionId)) {
        closeTab(chatId, sessionId);
      }
    }
  }

  void navigate(String chatId, String url) {
    final sessionId = _activeSessionIdByChatId[chatId];
    if (sessionId == null) {
      throw StateError('No active browser session for chat_id $chatId');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.navigate(url);
  }

  void navigateBack(String chatId) {
    final sessionId = _activeSessionIdByChatId[chatId];
    if (sessionId == null) {
      throw StateError('No active browser session for chat_id $chatId');
    }
    final controls = _sessionControls[sessionId];
    if (controls == null) {
      throw StateError('Browser session controls are not registered');
    }
    controls.navigateBack();
  }

  void register({
    required String chatId,
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
      chatId: chatId,
      sessionId: sessionId,
      title: title,
      url: url,
      active: active,
    );
    if (active) {
      _activeSessionId = sessionId;
      _activeSessionIdByChatId[chatId] = sessionId;
      _completeChatSessionWaiters(chatId);
    }
  }

  void update({
    required String chatId,
    required String sessionId,
    required String title,
    required String url,
    required bool active,
  }) {
    if (!_controllers.containsKey(sessionId)) {
      return;
    }
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      chatId: chatId,
      sessionId: sessionId,
      title: title,
      url: url,
      active: active,
    );
    if (active) {
      _activeSessionId = sessionId;
      _activeSessionIdByChatId[chatId] = sessionId;
      _completeChatSessionWaiters(chatId);
    }
  }

  void unregister(String sessionId) {
    final session = _sessions[sessionId];
    _controllers.remove(sessionId);
    _sessionControls.remove(sessionId);
    _sessions.remove(sessionId);
    if (_activeSessionId == sessionId) {
      _activeSessionId = _sessions.isEmpty ? null : _sessions.keys.last;
    }
    final chatId = session?.chatId;
    if (chatId != null && _activeSessionIdByChatId[chatId] == sessionId) {
      final remaining = _sessions.values
          .where((item) => item.chatId == chatId)
          .map((item) => item.sessionId)
          .toList(growable: false);
      if (remaining.isEmpty) {
        _activeSessionIdByChatId.remove(chatId);
      } else {
        _activeSessionIdByChatId[chatId] = remaining.last;
      }
    }
  }

  void _completeChatSessionWaiters(String chatId) {
    final waiters = _chatSessionWaiters.remove(chatId);
    if (waiters == null) {
      return;
    }
    for (final waiter in waiters) {
      if (!waiter.isCompleted) {
        waiter.complete();
      }
    }
  }
}
