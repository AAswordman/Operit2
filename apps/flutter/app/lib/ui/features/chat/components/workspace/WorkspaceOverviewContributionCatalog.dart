// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../common/contributions/ToolPkgChatUiCatalog.dart';
import 'WorkspaceOverviewModels.dart';

/// Owns the request lifetime for the mounted workspace overview catalog.
class WorkspaceOverviewContributionCatalog extends ChangeNotifier {
  /// Starts with no successful catalog result until actual histories are available.
  WorkspaceOverviewContributionCatalog();

  WorkspaceContributionsState _state = const WorkspaceContributionsLoading();
  int _generation = 0;
  bool _disposed = false;

  /// Returns the current request state without exposing stale successful results.
  WorkspaceContributionsState get state => _state;

  /// Invalidates pending requests while the authoritative history stream is pending.
  void awaitHistories() {
    _ensureActive();
    ++_generation;
    _state = const WorkspaceContributionsLoading();
    notifyListeners();
  }

  /// Exposes history or scope errors and invalidates all older catalog requests.
  void reportError(Object error, StackTrace stackTrace) {
    _ensureActive();
    ++_generation;
    _state = WorkspaceContributionsFailed(error, stackTrace);
    notifyListeners();
  }

  /// Loads the existing catalog with an immutable snapshot of the actual workspace.
  Future<void> refresh({
    required ToolPkgChatUiCatalog catalog,
    required List<core_proxy.ChatHistoryListItem> chats,
  }) async {
    _ensureActive();
    final generation = ++_generation;
    final summaries = List<core_proxy.ChatHistoryListItem>.unmodifiable(chats);
    _state = const WorkspaceContributionsLoading();
    notifyListeners();
    try {
      final sections = await catalog.loadSections(chats: summaries);
      if (_disposed || generation != _generation) {
        return;
      }
      _state = WorkspaceContributionsReady(
        workspaceContributionUsages(sections),
      );
    } catch (error, stackTrace) {
      if (_disposed || generation != _generation) {
        return;
      }
      _state = WorkspaceContributionsFailed(error, stackTrace);
    }
    notifyListeners();
  }

  /// Rejects new work after the owning panel has released this catalog state.
  void _ensureActive() {
    if (_disposed) {
      throw StateError('The workspace overview catalog is disposed.');
    }
  }

  /// Prevents every in-flight success or failure from notifying a disposed panel.
  @override
  void dispose() {
    _disposed = true;
    ++_generation;
    super.dispose();
  }
}
