// ignore_for_file: file_names

part of 'DrawerContent.dart';

extension _DesktopWorkspaceContent on _DrawerContentState {
  /// Uses canonical activity times across projects, independently of manual order.
  List<core_proxy.ChatHistoryListItem> _desktopRecentHistories(
    List<_WorkspaceSection> sections,
  ) {
    final visibleIds = {
      for (final section in sections)
        for (final history in section.histories) history.id,
    };
    final recent = widget.histories.indexed
        .where((entry) => visibleIds.contains(entry.$2.id))
        .map(
          (entry) => (
            history: entry.$2,
            updatedAt:
                DateTime.tryParse(entry.$2.updatedAt)?.microsecondsSinceEpoch ??
                0,
            originalOrder: entry.$1,
          ),
        )
        .toList();
    recent.sort((left, right) {
      final activity = right.updatedAt.compareTo(left.updatedAt);
      return activity != 0
          ? activity
          : left.originalOrder.compareTo(right.originalOrder);
    });
    return recent.map((entry) => entry.history).toList(growable: false);
  }

  Widget _desktopRecentHistory(
    List<core_proxy.ChatHistoryListItem> histories,
    bool searching,
  ) {
    final appearance = widget.appearance;
    final open = searching || !_recentHistoryCollapsed;
    final expanded = searching || _recentHistoryExpanded;
    return Column(
      key: const ValueKey('recent-history-section'),
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Padding(
          padding: const EdgeInsetsDirectional.fromSTEB(14, 2, 12, 0),
          child: InkWell(
            key: const ValueKey('recent-history-header'),
            borderRadius: BorderRadius.circular(8),
            onTap: _toggleRecentHistory,
            child: Padding(
              padding: const EdgeInsetsDirectional.fromSTEB(8, 4, 6, 4),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      '最近会话',
                      style: Theme.of(context).textTheme.labelLarge?.copyWith(
                        color: appearance.titleColor,
                        fontSize: 12.5,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  Icon(
                    open ? Icons.expand_less : Icons.expand_more,
                    size: 18,
                    color: appearance.itemColor.withValues(alpha: 0.7),
                  ),
                ],
              ),
            ),
          ),
        ),
        if (open) ...<Widget>[
          for (final history
              in expanded
                  ? histories
                  : histories.take(_DrawerContentState._previewLimit))
            _historyRow(history, recent: true),
          if (!searching &&
              histories.length > _DrawerContentState._previewLimit)
            _HistoryLimitButton(
              key: const ValueKey('recent-history-limit'),
              icon: expanded ? Icons.expand_less : Icons.expand_more,
              label: expanded
                  ? '收起'
                  : '展开更多 ${histories.length - _DrawerContentState._previewLimit}',
              workspaceStyle: true,
              flat: true,
              appearance: appearance,
              onClick: _toggleRecentPreview,
            ),
        ],
      ],
    );
  }

  /// Keeps creation above a single list of expanded projects and their groups.
  Widget _desktopWorkspaceContent(BuildContext context, {Widget? tabs}) {
    final sections = _workspaceSections();
    final recent = _desktopRecentHistories(sections);
    final searching = _searchController.text.trim().isNotEmpty;
    final error = _errorMessage ?? widget.errorMessage;
    final appearance = widget.appearance;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        if (tabs != null) _sidebarHeader(tabs),
        RepaintBoundary(
          key: const ValueKey('workspace-create-toolbar'),
          child: Padding(
            padding: const EdgeInsetsDirectional.fromSTEB(14, 8, 12, 8),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: _WorkspaceCreateBar(
                    onCreateConversation: () =>
                        unawaited(_createConversation(inheritWorkspace: true)),
                  ),
                ),
                const SizedBox(width: 8),
                _ToolbarIconButton(
                  icon: _searchExpanded
                      ? Icons.search_off_rounded
                      : Icons.search_rounded,
                  tooltip: _searchExpanded ? '收起搜索' : '搜索对话',
                  appearance: appearance,
                  active: _searchExpanded,
                  onClick: _toggleSearchExpanded,
                ),
              ],
            ),
          ),
        ),
        AnimatedSize(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutCubic,
          child: _searchExpanded
              ? Padding(
                  padding: const EdgeInsetsDirectional.fromSTEB(14, 0, 12, 8),
                  child: ConversationSearchField(
                    controller: _searchController,
                    appearance: appearance,
                    hintText: '搜索所有项目的对话',
                  ),
                )
              : const SizedBox.shrink(),
        ),
        if (error != null)
          SidebarStatusText(text: error, appearance: appearance),
        Expanded(
          child: Stack(
            children: <Widget>[
              CustomScrollView(
                controller: _historyScrollController,
                key: const PageStorageKey<String>('desktop-workspace-scroll'),
                primary: false,
                slivers: <Widget>[
                  if (recent.isNotEmpty)
                    SliverToBoxAdapter(
                      child: _desktopRecentHistory(recent, searching),
                    ),
                  for (final section in sections) ...[
                    SliverToBoxAdapter(
                      child: Padding(
                        padding: EdgeInsets.only(
                          top: section == sections.first && recent.isEmpty
                              ? 2
                              : 12,
                        ),
                        child: _WorkspaceHeader(
                          key: ValueKey('workspace-header:${section.key}'),
                          label: section.label,
                          subtitle: section.subtitle,
                          unbound: section.workspaceId == null,
                          flat: true,
                          expanded:
                              searching ||
                              !_collapsedWorkspaces.contains(section.key),
                          pinned: _pinnedDirectories.contains(
                            SidebarDirectory.workspace(section.workspaceId),
                          ),
                          appearance: appearance,
                          canCreate: section.canCreate,
                          onToggleExpanded: () => _toggleWorkspace(section.key),
                          onCreateConversation: () => unawaited(
                            _createConversation(
                              workspaceId: section.workspaceId,
                            ),
                          ),
                          onCreateGroup: () =>
                              unawaited(_createWorkspaceGroup(section)),
                          onTogglePinned: () => unawaited(
                            _toggleDirectoryPinned(
                              SidebarDirectory.workspace(section.workspaceId),
                            ),
                          ),
                          onDelete:
                              section.workspaceId != null && section.canCreate
                              ? () => unawaited(_deleteWorkspace(section))
                              : null,
                        ),
                      ),
                    ),
                    if (searching ||
                        !_collapsedWorkspaces.contains(section.key))
                      SliverToBoxAdapter(
                        child: section.histories.isEmpty
                            ? SidebarStatusText(
                                text: '暂无会话',
                                appearance: appearance,
                              )
                            : _workspaceHistory(section, searching),
                      ),
                  ],
                  if (sections.isEmpty && !widget.loading)
                    SliverToBoxAdapter(
                      child: SidebarStatusText(
                        text: searching ? '没有匹配的会话' : '暂无会话',
                        appearance: appearance,
                      ),
                    ),
                  const SliverToBoxAdapter(child: SizedBox(height: 16)),
                ],
              ),
              if (widget.loading && widget.histories.isEmpty && error == null)
                Positioned.fill(
                  child: IgnorePointer(
                    child: Center(
                      child: M3LoadingIndicator(
                        color: appearance.statusAvailableColor,
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

/// Connects group members with one light guide beneath the folder icon.
class _DesktopGroupBody extends StatelessWidget {
  const _DesktopGroupBody({required this.appearance, required this.child});

  final NavigationDrawerAppearance appearance;
  final Widget child;

  @override
  Widget build(BuildContext context) => Stack(
    clipBehavior: Clip.none,
    children: <Widget>[
      PositionedDirectional(
        start: 28,
        top: -4,
        bottom: 6,
        width: 1,
        child: ColoredBox(color: appearance.itemColor.withValues(alpha: 0.12)),
      ),
      child,
    ],
  );
}
