// ignore_for_file: file_names

import 'WorkspaceSession.dart';

/// Retains workspace presentation only in this frontend's process memory.
class WorkspaceSessionStore {
  /// Creates an empty local navigation store.
  WorkspaceSessionStore();

  final Map<(String?, String?), WorkspaceSession> _sessions = {};
  WorkspaceSession? _current;

  /// Returns the active session, initially the unbound new-chat workspace.
  WorkspaceSession get current =>
      _current ??= select(workspacePath: null, chatId: null);

  /// Shares bound workspaces across chats and isolates unbound chats by identity.
  WorkspaceSession select({
    required String? workspacePath,
    required String? chatId,
  }) {
    final path = workspacePath?.trim();
    final key = path == null || path.isEmpty ? (null, chatId) : (path, null);
    return _current = _sessions.putIfAbsent(key, WorkspaceSession.new);
  }
}
