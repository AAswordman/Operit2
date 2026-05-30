// ignore_for_file: file_names

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

class WorkspaceBrowserSessionRegistry {
  WorkspaceBrowserSessionRegistry._();

  static final WorkspaceBrowserSessionRegistry instance =
      WorkspaceBrowserSessionRegistry._();

  final Map<String, WorkspaceBrowserAutomationController> _controllers =
      <String, WorkspaceBrowserAutomationController>{};
  final Map<String, WorkspaceBrowserSessionInfo> _sessions =
      <String, WorkspaceBrowserSessionInfo>{};
  void Function(String url)? _createTab;
  void Function(String sessionId)? _selectTab;
  void Function(String sessionId)? _closeTab;
  void Function(String url)? _navigate;
  void Function()? _navigateBack;
  String? _activeSessionId;

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

  void setControls({
    required void Function(String url) createTab,
    required void Function(String sessionId) selectTab,
    required void Function(String sessionId) closeTab,
    required void Function(String url) navigate,
    required void Function() navigateBack,
  }) {
    _createTab = createTab;
    _selectTab = selectTab;
    _closeTab = closeTab;
    _navigate = navigate;
    _navigateBack = navigateBack;
  }

  void clearControls() {
    _createTab = null;
    _selectTab = null;
    _closeTab = null;
    _navigate = null;
    _navigateBack = null;
  }

  List<Map<String, Object?>> listTabs() {
    return sessions.map((session) => session.toJson()).toList(growable: false);
  }

  void createTab(String url) {
    _createTab?.call(url);
  }

  void selectTab(String sessionId) {
    _selectTab?.call(sessionId);
  }

  void closeTab(String sessionId) {
    _closeTab?.call(sessionId);
  }

  void navigate(String url) {
    _navigate?.call(url);
  }

  void navigateBack() {
    _navigateBack?.call();
  }

  void register({
    required String sessionId,
    required WorkspaceBrowserAutomationController controller,
    required String title,
    required String url,
    required bool active,
  }) {
    _controllers[sessionId] = controller;
    _sessions[sessionId] = WorkspaceBrowserSessionInfo(
      sessionId: sessionId,
      title: title,
      url: url,
      active: active,
    );
    if (active) {
      _activeSessionId = sessionId;
    }
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
      active: active,
    );
    if (active) {
      _activeSessionId = sessionId;
    }
  }

  void unregister(String sessionId) {
    _controllers.remove(sessionId);
    _sessions.remove(sessionId);
    if (_activeSessionId == sessionId) {
      _activeSessionId = _sessions.isEmpty ? null : _sessions.keys.last;
    }
  }
}
