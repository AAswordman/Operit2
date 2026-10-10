// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';
import '../../common/contributions/ChatSidebarTabHost.dart';
import '../../features/chat/viewmodel/ChatSelectionTransition.dart';
import '../../features/chat/components/NewChatIntro.dart';
import '../navigation/AppNavigationModels.dart';
import '../layout/SidebarDockController.dart';
import '../layout/NavigationLayoutMetrics.dart';
import '../screens/ScreenRouteRegistry.dart';
import '../../theme/OperitTheme.dart';
import '../../window/DetachedChatWindowLauncher.dart';
import '../../window/OperitWindowPlatform.dart';
import 'CollapsedDrawerContent.dart';
import '../../common/components/M3LoadingIndicator.dart';
import 'DrawerContentDialogs.dart';
import 'NavigationDrawerAppearance.dart';

part 'DrawerContentGroups.dart';

class DrawerContent extends StatefulWidget {
  /// Receives generic chat metadata; plugin tabs own all subject-specific views.
  const DrawerContent({
    super.key,
    required this.navigationEntries,
    required this.pluginEntries,
    required this.selectedRouteId,
    required this.appearance,
    required this.histories,
    required this.activeStreamingChatIds,
    required this.currentChatId,
    required this.errorMessage,
    required this.loading,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
    this.bridge = const ProxyCoreRuntimeBridge(),
  });

  final List<NavigationEntrySpec> navigationEntries;
  final List<NavigationEntrySpec> pluginEntries;
  final String selectedRouteId;
  final NavigationDrawerAppearance appearance;
  final List<core_proxy.ChatHistoryListItem> histories;
  final Set<String> activeStreamingChatIds;
  final String? currentChatId;
  final String? errorMessage;
  final bool loading;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;
  final OperitRuntimeBridge bridge;

  /// Creates the native workspace consumer and real registered-tab surface.
  @override
  State<DrawerContent> createState() => _DrawerContentState();
}

class _DrawerContentState extends State<DrawerContent> {
  static const int _previewLimit = 4;
  static final Set<String> _rememberedCollapsedWorkspaces = <String>{};
  final ScrollController _historyScrollController = ScrollController();
  final TextEditingController _searchController = TextEditingController();
  final Set<String> _collapsedWorkspaces = Set<String>.of(
    _rememberedCollapsedWorkspaces,
  );
  final Set<String> _expandedWorkspaces = <String>{};
  bool _searchExpanded = false;
  String? _errorMessage;
  final Set<String> _collapsedGroups = {};
  bool _groupMutationPending = false;

  /// Uses the actual receiving bridge for native conversation operations.
  GeneratedCoreProxyClients get _clients =>
      GeneratedCoreProxyClients(widget.bridge);

  /// Returns the shared canonical chat runtime without creating business managers.
  GeneratedChatRuntimeHolderMainCoreProxy get _chatCoreProxy =>
      _clients.chatRuntimeHolderMain;

  /// Observes native workspace search independently of plugin-owned tab state.
  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
  }

  /// Clears local errors only after fresh canonical drawer state arrives.
  @override
  void didUpdateWidget(covariant DrawerContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.errorMessage != widget.errorMessage) _errorMessage = null;
  }

  /// Releases native controllers without modifying persisted plugin tab selection.
  @override
  void dispose() {
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    _historyScrollController.dispose();
    super.dispose();
  }

  /// Rebuilds the native workspace projection when the search query changes.
  void _onSearchChanged() => setState(() {});

  /// Toggles only the native search control, not plugin registration parameters.
  void _toggleSearchExpanded() =>
      setState(() => _searchExpanded = !_searchExpanded);

  /// Opens an existing navigation entry using its exact registered host identity.
  void _openNavigation(String id) {
    final matches = widget.navigationEntries.where(
      (entry) => entry.entryId == id,
    );
    if (matches.length != 1) {
      throw StateError('Unknown or ambiguous navigation entry: $id');
    }
    widget.onNavigationEntrySelected(matches.single);
  }

  /// Runs the original switch, navigation and drawer-close chain after tab validation.
  Future<void> _activateChat(String chatId) async {
    final switchStartedAt = Stopwatch()..start();
    if (mounted) setState(() => _errorMessage = null);
    try {
      if (chatId != widget.currentChatId) ChatSelectionTransition.begin(chatId);
      widget.onConversationActivated();
      await WidgetsBinding.instance.endOfFrame;
      await _chatCoreProxy.switchChat(chatId: chatId);
      ClientLogger.i(
        'chat_switch.command_completed chatId=$chatId elapsedMs=${switchStartedAt.elapsedMilliseconds}',
        tag: 'ChatSwitchTrace',
      );
    } catch (error, stackTrace) {
      ChatSelectionTransition.complete(chatId);
      debugPrint('Failed to switch chat: $error\n$stackTrace');
      if (mounted) setState(() => _errorMessage = error.toString());
      rethrow;
    }
  }

  /// Creates through the generic hook using the displayed chat as the nullable source.
  Future<void> _createConversation() async {
    final sourceChatId = widget.currentChatId;
    final chatCore = _chatCoreProxy;
    final onActivated = widget.onConversationActivated;
    newChatIntroArmed.value = true;
    if (mounted) setState(() => _errorMessage = null);
    try {
      await chatCore.createNewChat(
        setAsCurrentChat: true,
        sourceChatId: sourceChatId,
        input: null,
      );
      onActivated();
    } catch (error) {
      newChatIntroArmed.value = false;
      if (mounted) {
        setState(() => _errorMessage = error.toString());
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  /// Reports native row failures while keeping the selected registered tab intact.
  Future<void> _switchConversation(
    core_proxy.ChatHistoryListItem history,
  ) async {
    try {
      await _activateChat(history.id);
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    }
  }

  /// Presents the retained generic conversation-title editor.
  Future<void> _showRenameConversationDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    final title = await showDialog<String>(
      context: context,
      useRootNavigator: true,
      builder: (context) {
        return RenameConversationDialog(history: history);
      },
    );
    if (!mounted || title == null) {
      return;
    }
    await _updateConversationTitle(history, title);
  }

  /// Confirms canonical conversation deletion without managing plugin groups.
  Future<void> _showDeleteConversationDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    if (history.locked) {
      await _deleteConversation(history);
      return;
    }
    final confirmed = await showDialog<bool>(
      context: context,
      useRootNavigator: true,
      builder: (context) {
        return DeleteConversationDialog(history: history);
      },
    );
    if (!mounted || confirmed != true) {
      return;
    }
    await _deleteConversation(history);
  }

  /// Presents native workspace actions for a canonical conversation row.
  Future<void> _showConversationActionDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    final canMoveUp = _canMoveConversationRelative(history, -1);
    final canMoveDown = _canMoveConversationRelative(history, 1);
    final action = await showDialog<ConversationAction>(
      context: context,
      useRootNavigator: true,
      builder: (context) {
        return ConversationActionDialog(
          history: history,
          canOpenInWindow: operitSupportsDesktopMultiWindow,
          canMoveUp: canMoveUp,
          canMoveDown: canMoveDown,
        );
      },
    );
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case ConversationAction.openInWindow:
        await DetachedChatWindowLauncher.openChat(
          chatId: history.id,
          title: history.title,
          themePreferenceSnapshot: OperitTheme.of(
            context,
          ).themePreferenceSnapshot,
        );
      case ConversationAction.rename:
        await _showRenameConversationDialog(history);
      case ConversationAction.moveUp:
        await _moveConversationRelative(history, -1);
      case ConversationAction.moveDown:
        await _moveConversationRelative(history, 1);
      case ConversationAction.togglePinned:
        await _updateConversationPinned(history);
      case ConversationAction.toggleLocked:
        await _updateConversationLocked(history);
      case ConversationAction.delete:
        await _showDeleteConversationDialog(history);
    }
  }

  /// Deletes a conversation and reports a policy refusal in the drawer.
  Future<void> _deleteConversation(
    core_proxy.ChatHistoryListItem history,
  ) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      final deleted = await _chatCoreProxy.deleteChatHistory(
        chatId: history.id,
      );
      if (deleted || !mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      setState(() {
        _errorMessage = l10n.chatLockedCannotDelete;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to delete chat history: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  /// Commits only the native conversation title through its existing runtime operation.
  Future<void> _updateConversationTitle(
    core_proxy.ChatHistoryListItem history,
    String title,
  ) async {
    final normalizedTitle = title.trim();
    if (normalizedTitle.isEmpty || normalizedTitle == history.title) {
      return;
    }
    setState(() {
      _errorMessage = null;
    });
    try {
      await _chatCoreProxy.updateChatTitle(
        chatId: history.id,
        title: normalizedTitle,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update chat title: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _updateConversationPinned(
    core_proxy.ChatHistoryListItem history,
  ) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      await _chatCoreProxy.updateChatPinned(
        chatId: history.id,
        pinned: !history.pinned,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update chat pinned state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _updateConversationLocked(
    core_proxy.ChatHistoryListItem history,
  ) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      await _chatCoreProxy.updateChatLocked(
        chatId: history.id,
        locked: !history.locked,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update chat locked state: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  /// Keeps the legacy native workspace identity and unbound fallback.
  String _workspaceKey(core_proxy.ChatHistoryListItem history) {
    final id = history.workspaceId?.trim();
    return id == null || id.isEmpty ? 'workspace:unbound' : 'workspace:$id';
  }

  String _workspaceLabel(core_proxy.ChatHistoryListItem history) {
    final name = history.workspaceName?.trim();
    return name == null || name.isEmpty ? '未绑定工作区' : name;
  }

  /// Projects the full canonical order into native workspace sections and search matches.
  List<_WorkspaceSection> _workspaceSections({bool filter = true}) {
    final query = _searchController.text.trim().toLowerCase();
    final sections = <String, _WorkspaceSection>{};
    for (final history in widget.histories) {
      final label = _workspaceLabel(history);
      if (filter &&
          query.isNotEmpty &&
          !history.title.toLowerCase().contains(query) &&
          !label.toLowerCase().contains(query) &&
          !(history.group ?? '未分组').toLowerCase().contains(query)) {
        continue;
      }
      final key = _workspaceKey(history);
      final section = sections.putIfAbsent(
        key,
        () => _WorkspaceSection(key: key, label: label),
      );
      section.histories.add(history);
    }
    return sections.values.toList(growable: false);
  }

  void _showGroupError(Object error) =>
      setState(() => _errorMessage = error.toString());

  /// Remembers workspace expansion without retaining deleted domain grouping state.
  void _toggleWorkspace(String key) => setState(() {
    if (!_collapsedWorkspaces.remove(key)) _collapsedWorkspaces.add(key);
    _rememberedCollapsedWorkspaces
      ..clear()
      ..addAll(_collapsedWorkspaces);
  });

  /// Expands the native preview independently of its workspace header collapse state.
  void _togglePreview(String key) => setState(() {
    if (!_expandedWorkspaces.remove(key)) _expandedWorkspaces.add(key);
  });

  /// Limits native reordering to conversations within the same workspace.
  bool _canMoveConversationRelative(
    core_proxy.ChatHistoryListItem history,
    int delta,
  ) {
    final index = widget.histories.indexWhere((item) => item.id == history.id);
    final target = index + delta;
    return index >= 0 &&
        target >= 0 &&
        target < widget.histories.length &&
        _workspaceKey(history) == _workspaceKey(widget.histories[target]);
  }

  /// Moves a native workspace row without reading a role or plugin selection field.
  Future<void> _moveConversationRelative(
    core_proxy.ChatHistoryListItem history,
    int delta,
  ) async {
    if (!_canMoveConversationRelative(history, delta)) return;
    final index = widget.histories.indexWhere((item) => item.id == history.id);
    final reordered = List<core_proxy.ChatHistoryListItem>.of(widget.histories);
    reordered.insert(index + delta, reordered.removeAt(index));
    await _saveOrder(reordered, history);
  }

  /// Accepts native row drops only inside the same workspace projection.
  Future<void> _moveConversationTo(
    core_proxy.ChatHistoryListItem moved,
    core_proxy.ChatHistoryListItem target,
  ) async {
    if (moved.id == target.id ||
        _workspaceKey(moved) != _workspaceKey(target)) {
      return;
    }
    if (moved.group != target.group) {
      await _moveWorkspaceGroup(moved, target.group);
    }
    final from = widget.histories.indexWhere((item) => item.id == moved.id);
    final to = widget.histories.indexWhere((item) => item.id == target.id);
    if (from < 0 || to < 0) {
      throw StateError('The reordered conversation is no longer present.');
    }
    final reordered = List<core_proxy.ChatHistoryListItem>.of(widget.histories);
    reordered.insert(to, reordered.removeAt(from));
    await _saveOrder(reordered, moved);
  }

  /// Persists native ordering through the generic canonical conversation operation.
  Future<void> _saveOrder(
    List<core_proxy.ChatHistoryListItem> reordered,
    core_proxy.ChatHistoryListItem moved,
  ) async {
    try {
      await _chatCoreProxy.updateChatOrder(
        reorderedHistories: reordered,
        movedItem: moved,
      );
    } catch (error) {
      if (mounted) setState(() => _errorMessage = error.toString());
    }
  }

  /// Builds a native row with the original pin, lock, title, delete and detached-window actions.
  Widget _historyRow(core_proxy.ChatHistoryListItem history) =>
      ConversationDrawerItem(
        key: ValueKey('workspace-chat:${history.id}'),
        history: history,
        title: history.title,
        selected:
            widget.selectedRouteId ==
                ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat) &&
            widget.currentChatId == history.id,
        isRunning: widget.activeStreamingChatIds.contains(history.id),
        appearance: widget.appearance,
        nested: true,
        workspaceStyle: true,
        onClick: () => unawaited(_switchConversation(history)),
        onRename: () => unawaited(_showRenameConversationDialog(history)),
        onTogglePinned: () => unawaited(_updateConversationPinned(history)),
        onToggleLocked: () => unawaited(_updateConversationLocked(history)),
        onDelete: () => unawaited(_showDeleteConversationDialog(history)),
        onLongPress: () => unawaited(_showConversationActionDialog(history)),
        canDetach: operitSupportsDesktopMultiWindow,
        onDetach: () => unawaited(
          DetachedChatWindowLauncher.openChat(
            chatId: history.id,
            title: history.title,
            themePreferenceSnapshot: OperitTheme.of(
              context,
            ).themePreferenceSnapshot,
          ).catchError((Object error, StackTrace stackTrace) {
            if (mounted) setState(() => _errorMessage = error.toString());
          }),
        ),
        onMoveTo: (moved) => unawaited(
          _runGroupMutation(() => _moveConversationTo(moved, history)),
        ),
        canAcceptDrop: (moved) =>
            _workspaceKey(moved) == _workspaceKey(history),
      );

  Widget _sidebarHeader(Widget tabs) => Padding(
    padding: const EdgeInsets.only(top: 26, right: 12),
    child: SidebarInfoCard(
      brandName: 'Operit',
      appearance: widget.appearance,
      trailing: tabs,
    ),
  );

  List<Widget> _pluginNavigationSlivers() => <Widget>[
    if (widget.pluginEntries.isNotEmpty) ...<Widget>[
      const SliverToBoxAdapter(child: SizedBox(height: 10)),
      SliverToBoxAdapter(
        child: Padding(
          padding: const EdgeInsetsDirectional.only(
            start: 28,
            end: 12,
            bottom: 2,
          ),
          child: Text(
            '插件',
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
              color: widget.appearance.titleColor.withValues(alpha: 0.82),
              fontWeight: FontWeight.w600,
            ),
          ),
        ),
      ),
      const SliverToBoxAdapter(child: SizedBox(height: 6)),
      SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          final entry = widget.pluginEntries[index];
          return PluginNavigationDrawerItem(
            entry: entry,
            selected: widget.selectedRouteId == entry.routeId,
            appearance: widget.appearance,
            onClick: () => widget.onNavigationEntrySelected(entry),
          );
        }, childCount: widget.pluginEntries.length),
      ),
      SliverToBoxAdapter(
        child: SidebarDockEndDropTarget(
          controller:
              MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint
              ? SidebarDockScope.maybeOf(context)
              : null,
          location: SidebarDockLocation.primary,
          height: 18,
        ),
      ),
    ],
  ];

  /// The workspace is always native; resolving membership never replaces its widgets.
  Widget _workspaceHistory(_WorkspaceSection section, bool searching) {
    final groups = _nativeGroups(section);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (final group in groups) ...[
          _GroupSectionHeader(
            key: ValueKey('workspace-group:${group.key}'),
            label: group.label,
            pinned:
                group.histories.isNotEmpty &&
                group.histories.every((chat) => chat.pinned),
            workspaceStyle: true,
            expanded: !_collapsedGroups.contains(group.key),
            appearance: widget.appearance,
            onToggleExpanded: () => setState(() {
              if (!_collapsedGroups.remove(group.key))
                _collapsedGroups.add(group.key);
            }),
            onCreateChat: () => unawaited(
              _runGroupMutation(
                () => _createGroupedConversation(section, group.name),
              ),
            ),
            onRename: () => unawaited(_renameWorkspaceGroup(section, group)),
            onTogglePinned: () => unawaited(
              _runGroupMutation(() async {
                final pinned =
                    group.histories.isNotEmpty &&
                    group.histories.every((chat) => chat.pinned);
                for (final chat in group.histories) {
                  await _chatCoreProxy.updateChatPinned(
                    chatId: chat.id,
                    pinned: !pinned,
                  );
                }
              }),
            ),
            onDelete: () => unawaited(_deleteWorkspaceGroup(group)),
            canAcceptDrop: (chat) => _workspaceKey(chat) == section.key,
            onMoveToGroup: (chat) => unawaited(
              _runGroupMutation(() => _moveWorkspaceGroup(chat, group.name)),
            ),
          ),
          if (!_collapsedGroups.contains(group.key)) ...[
            for (final chat
                in searching || _expandedWorkspaces.contains(group.key)
                    ? group.histories
                    : _groupPreview(group.histories))
              _historyRow(chat),
            if (!searching &&
                _groupPreview(group.histories).length < group.histories.length)
              _HistoryLimitButton(
                key: ValueKey('history-limit:${group.key}'),
                icon: _expandedWorkspaces.contains(group.key)
                    ? Icons.expand_less
                    : Icons.expand_more,
                label: _expandedWorkspaces.contains(group.key)
                    ? '收起'
                    : '展开更多 ${group.histories.length - _groupPreview(group.histories).length}',
                workspaceStyle: true,
                appearance: widget.appearance,
                onClick: () => _togglePreview(group.key),
              ),
          ],
        ],
      ],
    );
  }

  /// Uses the original native sliver layout, with no plugin UI route or loading substitution.
  Widget _workspaceContent(BuildContext context, {Widget? tabs}) {
    final sections = _workspaceSections();
    final searching = _searchController.text.trim().isNotEmpty;
    final error = _errorMessage ?? widget.errorMessage;
    return Stack(
      children: <Widget>[
        CustomScrollView(
          controller: _historyScrollController,
          key: const PageStorageKey<String>('drawer-history-scroll'),
          primary: false,
          slivers: <Widget>[
            if (tabs != null) ...<Widget>[
              SliverToBoxAdapter(child: _sidebarHeader(tabs)),
              const SliverToBoxAdapter(child: SizedBox(height: 12)),
            ],
            SliverToBoxAdapter(
              child: RepaintBoundary(
                key: const ValueKey('workspace-create-toolbar'),
                child: Padding(
                  padding: const EdgeInsetsDirectional.only(
                    start: 14,
                    end: 12,
                    bottom: 8,
                  ),
                  child: Row(
                    children: <Widget>[
                      Expanded(
                        child: _WorkspaceCreateBar(
                          onCreateGroup: () =>
                              unawaited(_createWorkspaceGroup()),
                          onCreateConversation: () =>
                              unawaited(_createConversation()),
                        ),
                      ),
                      const SizedBox(width: 8),
                      _ToolbarIconButton(
                        icon: _searchExpanded
                            ? Icons.search_off_rounded
                            : Icons.search_rounded,
                        tooltip: _searchExpanded ? '收起搜索' : '搜索对话',
                        appearance: widget.appearance,
                        active: _searchExpanded || searching,
                        onClick: _toggleSearchExpanded,
                      ),
                    ],
                  ),
                ),
              ),
            ),
            SliverToBoxAdapter(
              child: AnimatedSize(
                duration: const Duration(milliseconds: 180),
                curve: Curves.easeOutCubic,
                child: _searchExpanded
                    ? Padding(
                        padding: const EdgeInsetsDirectional.only(
                          start: 12,
                          end: 12,
                          bottom: 12,
                        ),
                        child: ConversationSearchField(
                          controller: _searchController,
                          appearance: widget.appearance,
                        ),
                      )
                    : const SizedBox.shrink(),
              ),
            ),
            if (error != null)
              SliverToBoxAdapter(
                child: SidebarStatusText(
                  text: error,
                  appearance: widget.appearance,
                ),
              ),
            for (final section in sections) ...<Widget>[
              SliverToBoxAdapter(
                child: _WorkspaceHeader(
                  key: ValueKey('workspace-header:${section.key}'),
                  label: section.label,
                  count: section.histories.length,
                  expanded: !_collapsedWorkspaces.contains(section.key),
                  appearance: widget.appearance,
                  onToggleExpanded: () => _toggleWorkspace(section.key),
                ),
              ),
              if (!_collapsedWorkspaces.contains(section.key)) ...<Widget>[
                SliverToBoxAdapter(
                  child: _workspaceHistory(section, searching),
                ),
              ],
            ],
            if (sections.isEmpty && !widget.loading)
              SliverToBoxAdapter(
                child: SidebarStatusText(
                  text: searching ? '没有匹配的会话' : '暂无会话',
                  appearance: widget.appearance,
                ),
              ),
            ..._pluginNavigationSlivers(),
            const SliverToBoxAdapter(child: SizedBox(height: 16)),
          ],
        ),
        if (widget.loading && widget.histories.isEmpty && error == null)
          Positioned.fill(
            child: IgnorePointer(
              child: Center(
                child: M3LoadingIndicator(
                  color: widget.appearance.statusAvailableColor,
                ),
              ),
            ),
          ),
      ],
    );
  }

  /// Keeps global navigation outside plugin content while embedding the real tab host.
  @override
  Widget build(BuildContext context) {
    final theme = OperitTheme.of(context);
    return Column(
      children: <Widget>[
        Expanded(
          child: ChatSidebarTabHost(
            clients: _clients,
            chats: widget.histories,
            currentChatId: widget.currentChatId,
            activeStreamingChatIds: widget.activeStreamingChatIds,
            headerBuilder: (context, tabs) => Column(
              children: <Widget>[
                _sidebarHeader(tabs),
                const SizedBox(height: 12),
              ],
            ),
            workspaceBuilder: _workspaceContent,
            workspaceWithTabsBuilder: (context, tabs) =>
                _workspaceContent(context, tabs: tabs),
            pluginFooter: widget.pluginEntries.isEmpty
                ? null
                : ConstrainedBox(
                    constraints: const BoxConstraints(maxHeight: 160),
                    child: CustomScrollView(
                      shrinkWrap: true,
                      slivers: _pluginNavigationSlivers(),
                    ),
                  ),
            onActivateChat: _activateChat,
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(14, 8, 14, 16),
          child: Row(
            children: <Widget>[
              Expanded(
                child: BottomSidebarAction(
                  icon: Icons.inventory_2_outlined,
                  label: '包管理',
                  appearance: widget.appearance,
                  selected:
                      widget.selectedRouteId ==
                      ScreenRouteRegistry.routeIdOf(
                        ScreenRouteRegistry.packageManager,
                      ),
                  onClick: () => _openNavigation('main.package_manager'),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: BottomSidebarAction(
                  icon: Icons.settings_outlined,
                  label: '设置',
                  appearance: widget.appearance,
                  selected:
                      widget.selectedRouteId ==
                      ScreenRouteRegistry.routeIdOf(
                        ScreenRouteRegistry.settings,
                      ),
                  onClick: () => _openNavigation('main.settings'),
                ),
              ),
              const SizedBox(width: 8),
              Builder(
                builder: (buttonContext) => _BottomThemeToggleButton(
                  appearance: widget.appearance,
                  darkThemeActive: theme.isDark(context),
                  onToggle: () => theme.toggle(buttonContext),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _WorkspaceCreateBar extends StatelessWidget {
  const _WorkspaceCreateBar({
    required this.onCreateConversation,
    required this.onCreateGroup,
  });
  final VoidCallback onCreateConversation;
  final VoidCallback onCreateGroup;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final contentColor = scheme.onPrimaryContainer;
    return SizedBox(
      height: 34,
      child: Material(
        color: scheme.primaryContainer,
        shape: const StadiumBorder(),
        clipBehavior: Clip.antiAlias,
        child: Row(
          children: [
            Expanded(
              child: InkWell(
                key: const ValueKey('workspace-create-chat'),
                borderRadius: const BorderRadius.horizontal(
                  left: Radius.circular(17),
                ),
                hoverColor: contentColor.withValues(alpha: 0.08),
                focusColor: contentColor.withValues(alpha: 0.10),
                splashColor: contentColor.withValues(alpha: 0.10),
                highlightColor: contentColor.withValues(alpha: 0.10),
                onTap: onCreateConversation,
                child: Center(
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Icon(Icons.add_rounded, size: 17, color: contentColor),
                      const SizedBox(width: 6),
                      Text(
                        '新建对话',
                        style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          color: contentColor,
                          letterSpacing: -0.1,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            ...[
              Container(
                width: 1,
                height: 16,
                color: contentColor.withValues(alpha: 0.16),
              ),
              SizedBox(
                width: 38,
                height: 34,
                child: InkWell(
                  key: const ValueKey('workspace-create-group'),
                  onTap: onCreateGroup,
                  borderRadius: const BorderRadius.horizontal(
                    right: Radius.circular(17),
                  ),
                  child: Tooltip(
                    message: '新建分组',
                    child: Center(
                      child: Icon(
                        Icons.create_new_folder_outlined,
                        size: 16,
                        color: contentColor,
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _WorkspaceSection {
  /// Retains complete native conversation rows for count, collapse and search behavior.
  _WorkspaceSection({required this.key, required this.label});
  final String key;
  final String label;
  final List<core_proxy.ChatHistoryListItem> histories =
      <core_proxy.ChatHistoryListItem>[];
}

class _WorkspaceHeader extends StatelessWidget {
  /// Displays the native workspace header without subject-specific icons or avatars.
  const _WorkspaceHeader({
    super.key,
    required this.label,
    required this.count,
    required this.expanded,
    required this.appearance,
    required this.onToggleExpanded,
  });
  final String label;
  final int count;
  final bool expanded;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onToggleExpanded;

  /// Preserves native workspace collapse and complete conversation counts.
  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsetsDirectional.only(
      start: 18,
      end: 12,
      top: 8,
      bottom: 4,
    ),
    child: InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: onToggleExpanded,
      child: Padding(
        padding: const EdgeInsetsDirectional.fromSTEB(2, 3, 4, 3),
        child: Row(
          children: <Widget>[
            Container(
              width: 3,
              height: 17,
              decoration: BoxDecoration(
                color: appearance.statusAvailableColor.withValues(alpha: 0.62),
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            const SizedBox(width: 9),
            Icon(
              Icons.work_outline,
              size: 15,
              color: appearance.itemColor.withValues(alpha: 0.82),
            ),
            const SizedBox(width: 7),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: appearance.titleColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            const SizedBox(width: 8),
            _HistoryCountBadge(count: count, appearance: appearance),
            const SizedBox(width: 6),
            Icon(
              expanded ? Icons.expand_less : Icons.expand_more,
              size: 18,
              color: appearance.itemColor.withValues(alpha: 0.70),
            ),
          ],
        ),
      ),
    ),
  );
}

class _HistoryCountBadge extends StatelessWidget {
  /// Creates a compact count badge for history section rows.
  const _HistoryCountBadge({required this.count, required this.appearance});

  final int count;
  final NavigationDrawerAppearance appearance;

  /// Builds a small outlined count badge.
  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: const BoxConstraints(minWidth: 18),
      padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: appearance.dividerColor),
      ),
      alignment: Alignment.center,
      child: Text(
        count.toString(),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: Theme.of(context).textTheme.labelSmall?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.70),
          fontWeight: FontWeight.w700,
        ),
      ),
    );
  }
}

class _BottomThemeToggleButton extends StatelessWidget {
  const _BottomThemeToggleButton({
    required this.appearance,
    required this.darkThemeActive,
    required this.onToggle,
  });

  final NavigationDrawerAppearance appearance;
  final bool darkThemeActive;
  final VoidCallback onToggle;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(8);
    return Tooltip(
      message: darkThemeActive ? '切换白天模式' : '切换黑夜模式',
      child: SizedBox(
        width: 34,
        height: 34,
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: appearance.buttonContainerColor.withValues(alpha: 0.55),
            borderRadius: shape,
            border: Border.all(
              color: appearance.dividerColor.withValues(alpha: 0.35),
              width: 1,
            ),
          ),
          child: Material(
            color: Colors.transparent,
            borderRadius: shape,
            child: InkWell(
              borderRadius: shape,
              onTap: onToggle,
              child: Center(
                child: Icon(
                  darkThemeActive
                      ? Icons.light_mode_outlined
                      : Icons.dark_mode_outlined,
                  size: 16,
                  color: appearance.itemColor.withValues(alpha: 0.85),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ToolbarIconButton extends StatelessWidget {
  const _ToolbarIconButton({
    required this.icon,
    required this.tooltip,
    required this.appearance,
    required this.onClick,
    this.active = false,
  });

  final IconData icon;
  final String tooltip;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final bool active;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: SizedBox(
        width: 34,
        height: 34,
        child: Material(
          color: active
              ? appearance.selectedContainerColor.withValues(alpha: 0.35)
              : Colors.transparent,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            borderRadius: BorderRadius.circular(6),
            onTap: onClick,
            child: Center(
              child: Icon(
                icon,
                size: 17,
                color: active
                    ? appearance.statusAvailableColor
                    : appearance.itemColor.withValues(alpha: 0.78),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _HistoryLimitButton extends StatelessWidget {
  /// Creates the inline control for a group's conversation preview.
  const _HistoryLimitButton({
    super.key,
    required this.icon,
    required this.label,
    required this.workspaceStyle,
    required this.appearance,
    required this.onClick,
  });

  final IconData icon;
  final String label;
  final bool workspaceStyle;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;

  /// Renders the reversible preview toggle inline with its conversation group.
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsetsDirectional.only(
        start: workspaceStyle ? 51 : 56,
        end: 12,
        top: 2,
      ),
      child: TextButton.icon(
        onPressed: onClick,
        icon: Icon(
          icon,
          size: 18,
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
        label: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        style: TextButton.styleFrom(
          alignment: Alignment.centerLeft,
          foregroundColor: appearance.itemColor.withValues(alpha: 0.72),
          textStyle: Theme.of(
            context,
          ).textTheme.labelMedium?.copyWith(fontWeight: FontWeight.w600),
        ),
      ),
    );
  }
}
