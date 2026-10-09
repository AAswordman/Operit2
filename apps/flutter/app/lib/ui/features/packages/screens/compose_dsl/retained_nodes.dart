// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Collects node reads during a renderer build, including inline slot and layout metadata.
class _ComposeDslReadScope {
  static Set<_ComposeDslNode>? _active;

  /// Records an observed node in the current synchronous build scope.
  static void read(_ComposeDslNode node) { _active?.add(node); }

  /// Captures a build's node dependencies and restores its enclosing scope on every exit.
  static T track<T>(Set<_ComposeDslNode> reads, T Function() build) {
    final previous = _active;
    _active = reads;
    try { return build(); } finally { _active = previous; }
  }
}

/// Owns flat node handles and applies a commit before notifying any widget.
class _ComposeDslNodeStore {
  final Map<String, _ComposeDslNode> _nodes = {};
  String? _rootId;
  int _revision = 0;

  /// Resolves a required node and reports a malformed update immediately.
  _ComposeDslNode node(String id) {
    final node = _nodes[id];
    if (node == null) throw StateError('Compose update references an unknown node: $id');
    return node;
  }

  /// Returns the current retained root after at least one successful render commit.
  _ComposeDslNode get root => node(_rootId!);

  /// Applies ordered node changes without decoding JSON or rebuilding unchanged node models.
  void apply(core_proxy.ToolPkgComposeDslNodeUpdate update) {
    if (!update.reset && update.revision != _revision + 1) {
      throw StateError('Compose update revision is out of order: ${update.revision}, expected ${_revision + 1}');
    }
    if (update.reset) {
      for (final node in _nodes.values) { node.dispose(); }
      _nodes.clear();
    }
    for (final record in update.upserts) {
      _nodes.putIfAbsent(record.id, () => _ComposeDslNode(
        id: record.id, type: record.nodeType,
        props: const {}, children: const [], slots: const {},
      ));
    }
    for (final record in update.upserts) { node(record.id).replace(record, this); }
    _rootId = update.rootId;
    _revision = update.revision;
    node(update.rootId);
    for (final id in update.removed) {
      final removed = _nodes.remove(id);
      if (removed == null) throw StateError('Compose update removes an unknown node: $id');
      removed.dispose();
    }
    for (final record in update.upserts) { node(record.id).publish(); }
  }

  /// Releases all retained node listeners when the owning screen leaves the widget tree.
  void dispose() {
    for (final node in _nodes.values) { node.dispose(); }
    _nodes.clear();
  }
}

/// Correlates a command's completion and side effects without one subscription per action.
class _ComposeDslPendingCommand {
  /// Records the page generation and detached-event lifetime for one request.
  _ComposeDslPendingCommand(this.generation, {required this.keepDetachedEvents});
  final int generation;
  final bool keepDetachedEvents;
  final Completer<void> completion = Completer<void>();
  Object? actionResult;
  final List<({String routeId, Map<String, Object?> args})> navigationCommands = [];
}
