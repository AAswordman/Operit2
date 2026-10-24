// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Projects native parent relationships into ordered branches, including surviving orphan roots.
class ChatHistoryTree {
  /// Indexes each conversation once and rejects cycles before rendering recursive rows.
  ChatHistoryTree(List<core.ChatHistoryListItem> histories) {
    final byId = {for (final chat in histories) chat.id: chat};
    if (byId.length != histories.length) {
      throw StateError('Duplicate conversation id');
    }
    for (final chat in histories) {
      children[chat.id] = [];
    }
    for (final chat in histories) {
      final parent = chat.parentChatId;
      if (parent == null || !byId.containsKey(parent)) {
        roots.add(chat);
      } else {
        children[parent]!.add(chat);
      }
    }
    final visiting = <String>{}, visited = <String>{};

    /// Validates the native forest independently of its current expansion state.
    void visit(core.ChatHistoryListItem chat) {
      if (visited.contains(chat.id)) return;
      if (!visiting.add(chat.id)) {
        throw StateError('Cyclic conversation parent: ${chat.id}');
      }
      for (final child in children[chat.id]!) {
        visit(child);
      }
      visiting.remove(chat.id);
      visited.add(chat.id);
    }

    for (final chat in histories) {
      visit(chat);
    }
  }

  final roots = <core.ChatHistoryListItem>[];
  final children = <String, List<core.ChatHistoryListItem>>{};

  /// Retains ancestors of matching descendants so search can reveal their exact paths.
  Set<String> matching(bool Function(core.ChatHistoryListItem) predicate) {
    final result = <String>{};

    /// Evaluates every child even when its ancestor already matches.
    bool visit(core.ChatHistoryListItem chat) {
      var matches = predicate(chat);
      for (final child in children[chat.id]!) {
        matches = visit(child) || matches;
      }
      if (matches) result.add(chat.id);
      return matches;
    }

    for (final root in roots) {
      visit(root);
    }
    return result;
  }
}
