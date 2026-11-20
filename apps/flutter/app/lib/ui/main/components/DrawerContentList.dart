// ignore_for_file: file_names

part of 'DrawerContent.dart';

/// Describes one visible tree row without constructing its controls or render objects.
class _SidebarRow {
  /// Keeps identity independent of position when a directory expands or moves.
  const _SidebarRow(this.id, this.build);

  final String id;
  final Widget Function() build;
}

extension _LazyWorkspaceContent on _DrawerContentState {
  /// Shares one recycling sliver across projects, groups, branches, and recent chats.
  Widget _workspaceHistorySliver(
    List<_WorkspaceSection> sections,
    bool searching, {
    List<core_proxy.ChatHistoryListItem> recent = const [],
  }) {
    final rows = <_SidebarRow>[
      if (recent.isNotEmpty) ..._recentHistoryRows(recent, searching),
      for (var index = 0; index < sections.length; index++)
        ..._workspaceSectionRows(
          sections[index],
          searching,
          topPadding: widget.conversationsOnly
              ? (index == 0 && recent.isEmpty ? 2 : 12)
              : 0,
        ),
    ];
    final indices = <String, int>{
      for (var index = 0; index < rows.length; index++) rows[index].id: index,
    };
    return SliverList(
      key: const ValueKey('workspace-history-list'),
      delegate: SliverChildBuilderDelegate(
        (context, index) => KeyedSubtree(
          key: ValueKey(rows[index].id),
          child: rows[index].build(),
        ),
        childCount: rows.length,
        findChildIndexCallback: (key) =>
            key is ValueKey<String> ? indices[key.value] : null,
        addAutomaticKeepAlives: false,
      ),
    );
  }

  /// Flattens a project header and only its currently expanded groups.
  Iterable<_SidebarRow> _workspaceSectionRows(
    _WorkspaceSection section,
    bool searching, {
    required double topPadding,
  }) sync* {
    final expanded =
        searching && widget.conversationsOnly ||
        !_collapsedWorkspaces.contains(section.key);
    yield _SidebarRow(
      'header:${section.key}',
      () => Padding(
        padding: EdgeInsets.only(top: topPadding),
        child: _WorkspaceHeader(
          key: ValueKey('workspace-header:${section.key}'),
          label: section.label,
          subtitle: section.subtitle,
          unbound: section.workspaceId == null,
          flat: widget.conversationsOnly,
          expanded: expanded,
          pinned: _pinnedDirectories.contains(
            SidebarDirectory.workspace(section.workspaceId),
          ),
          appearance: widget.appearance,
          canCreate: section.canCreate,
          onToggleExpanded: () => _toggleWorkspace(section.key),
          onCreateConversation: () =>
              unawaited(_createConversation(workspaceId: section.workspaceId)),
          onCreateGroup: () => unawaited(_createWorkspaceGroup(section)),
          onTogglePinned: () => unawaited(
            _toggleDirectoryPinned(
              SidebarDirectory.workspace(section.workspaceId),
            ),
          ),
          onDelete: section.workspaceId == null
              ? null
              : () => unawaited(_deleteWorkspace(section)),
        ),
      ),
    );
    if (!expanded) return;
    if (section.histories.isEmpty) {
      if (widget.conversationsOnly) {
        yield _SidebarRow(
          'empty:${section.key}',
          () => SidebarStatusText(text: '暂无会话', appearance: widget.appearance),
        );
      }
      return;
    }
    final groups = _nativeGroups(section);
    for (var index = 0; index < groups.length; index++) {
      final group = groups[index];
      final open =
          searching && widget.conversationsOnly ||
          !_collapsedGroups.contains(group.key);
      final top = widget.conversationsOnly && index > 0 ? 4.0 : 0.0;
      yield _SidebarRow(
        'group:${group.key}',
        () => Padding(
          padding: EdgeInsets.only(top: top),
          child: _GroupSectionHeader(
            key: ValueKey('workspace-group:${group.key}'),
            group: group,
            pinned: _pinnedDirectories.contains(group.directory),
            expanded: open,
            compact: widget.conversationsOnly,
            appearance: widget.appearance,
            canCreate: section.canCreate,
            onToggleExpanded: () => _toggleGroup(group.key),
            onCreateChat: () => unawaited(
              _runGroupMutation(
                () => _createGroupedConversation(section, group.name),
              ),
            ),
            onCreateGroup: () => unawaited(_createWorkspaceGroup(section)),
            onRename: () => unawaited(_renameWorkspaceGroup(section, group)),
            onTogglePinned: () =>
                unawaited(_toggleDirectoryPinned(group.directory)),
            onDissolve: () =>
                unawaited(_dissolveWorkspaceGroup(section, group)),
            canAcceptDrop: (chat) => _workspaceKey(chat) == section.key,
            onMoveToGroup: (chat) => unawaited(
              _runGroupMutation(() => _moveWorkspaceGroup(chat, group.name)),
            ),
          ),
        ),
      );
      if (open) yield* _workspaceGroupRows(group, searching);
    }
  }

  /// Keeps expansion in the drawer owner while offscreen conversation widgets are disposed.
  Iterable<_SidebarRow> _workspaceGroupRows(
    _NativeWorkspaceGroup group,
    bool searching,
  ) sync* {
    final tree = ChatHistoryTree(group.histories);
    final retained = tree.matching(
      (chat) =>
          chat.pinned ||
          chat.id == widget.currentChatId ||
          widget.activeStreamingChatIds.contains(chat.id),
    );
    final preview = [
      for (var index = 0; index < tree.roots.length; index++)
        if (index < _DrawerContentState._previewLimit ||
            retained.contains(tree.roots[index].id))
          tree.roots[index],
    ];
    final expanded = _expandedWorkspaces.contains(group.key);
    final rows = <_SidebarRow>[];

    /// Appends lightweight row factories for expanded descendants in tree order.
    void append(core_proxy.ChatHistoryListItem chat, int depth) {
      final children = tree.children[chat.id]!;
      final open = searching || _expandedChatBranches.contains(chat.id);
      rows.add(
        _SidebarRow(
          'chat:${chat.id}',
          () => Padding(
            padding: EdgeInsetsDirectional.only(
              start: (widget.conversationsOnly ? 20.0 : 16.0) + depth * 16,
            ),
            child: _historyRow(
              chat,
              childCount: children.length,
              childrenExpanded: open,
            ),
          ),
        ),
      );
      if (open) {
        for (final child in children) {
          append(child, depth + 1);
        }
      }
    }

    for (final chat in searching || expanded ? tree.roots : preview) {
      append(chat, 0);
    }
    if (!searching && preview.length < tree.roots.length) {
      rows.add(
        _SidebarRow(
          'limit:${group.key}',
          () => Padding(
            padding: EdgeInsetsDirectional.only(
              start: widget.conversationsOnly ? 20 : 0,
            ),
            child: _HistoryLimitButton(
              key: ValueKey('history-limit:${group.key}'),
              icon: expanded ? Icons.expand_less : Icons.expand_more,
              label: expanded
                  ? '收起'
                  : '展开更多 ${tree.roots.length - preview.length}',
              workspaceStyle: true,
              flat: widget.conversationsOnly,
              appearance: widget.appearance,
              onClick: () => _togglePreview(group.key),
            ),
          ),
        ),
      );
    }
    for (var index = 0; index < rows.length; index++) {
      final row = rows[index];
      final first = index == 0, last = index == rows.length - 1;
      yield _SidebarRow(
        row.id,
        () => widget.conversationsOnly
            ? _DesktopGroupBody(
                appearance: widget.appearance,
                first: first,
                last: last,
                child: row.build(),
              )
            : row.build(),
      );
    }
  }

  /// Virtualizes recent shortcuts independently of the same chats in their projects.
  Iterable<_SidebarRow> _recentHistoryRows(
    List<core_proxy.ChatHistoryListItem> histories,
    bool searching,
  ) sync* {
    yield _SidebarRow('recent:header', _desktopRecentHeader);
    if (!searching && _recentHistoryCollapsed) return;
    final expanded = searching || _recentHistoryExpanded;
    for (final chat
        in expanded
            ? histories
            : histories.take(_DrawerContentState._previewLimit)) {
      yield _SidebarRow(
        'recent:${chat.id}',
        () => _historyRow(chat, recent: true),
      );
    }
    if (!searching && histories.length > _DrawerContentState._previewLimit) {
      yield _SidebarRow(
        'recent:limit',
        () => _HistoryLimitButton(
          key: const ValueKey('recent-history-limit'),
          icon: expanded ? Icons.expand_less : Icons.expand_more,
          label: expanded
              ? '收起'
              : '展开更多 ${histories.length - _DrawerContentState._previewLimit}',
          workspaceStyle: true,
          flat: true,
          appearance: widget.appearance,
          onClick: _toggleRecentPreview,
        ),
      );
    }
  }
}
