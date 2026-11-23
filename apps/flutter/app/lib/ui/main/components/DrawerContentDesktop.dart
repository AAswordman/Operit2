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

  /// Builds only the recent-section header; conversation rows belong to the lazy sliver.
  Widget _desktopRecentHeader() {
    final appearance = widget.appearance;
    final open =
        _searchController.text.trim().isNotEmpty || !_recentHistoryCollapsed;
    return Padding(
      key: const ValueKey('recent-history-section'),
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
            padding: const EdgeInsetsDirectional.fromSTEB(14, 12, 12, 8),
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
                  _workspaceHistorySliver(sections, searching, recent: recent),
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
  /// Draws one segment of a group guide while its row is materialized.
  const _DesktopGroupBody({
    required this.appearance,
    required this.child,
    required this.first,
    required this.last,
  });

  final NavigationDrawerAppearance appearance;
  final Widget child;
  final bool first;
  final bool last;

  /// Joins adjacent row segments without retaining an offscreen group column.
  @override
  Widget build(BuildContext context) => Stack(
    clipBehavior: Clip.none,
    children: <Widget>[
      PositionedDirectional(
        start: 28,
        top: first ? -4 : 0,
        bottom: last ? 6 : 0,
        width: 1,
        child: ColoredBox(color: appearance.itemColor.withValues(alpha: 0.12)),
      ),
      child,
    ],
  );
}
