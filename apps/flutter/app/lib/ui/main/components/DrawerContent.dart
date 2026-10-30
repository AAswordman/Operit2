// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/logging/ClientLogger.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';
import '../../common/contributions/ChatSidebarTabHost.dart';
import '../../features/chat/viewmodel/ChatSelectionTransition.dart';
import '../../features/chat/viewmodel/ChatMessageLookupRequests.dart';
import '../../features/chat/components/NewChatIntro.dart';
import '../../features/chat/components/workspace/WorkspaceUnbindDialog.dart';
import '../navigation/AppNavigationModels.dart';
import '../layout/SidebarDockController.dart';
import '../layout/NavigationLayoutMetrics.dart';
import '../layout/SidebarDockPreferences.dart';
import '../screens/ScreenRouteRegistry.dart';
import '../../theme/OperitTheme.dart';
import '../../window/DetachedChatWindowLauncher.dart';
import '../../window/OperitWindowPlatform.dart';
import 'CollapsedDrawerContent.dart';
import '../../common/components/M3LoadingIndicator.dart';
import 'DrawerContentDialogs.dart';
import 'NavigationDrawerAppearance.dart';
import 'SidebarMoreMenu.dart';
import 'SidebarChatCreation.dart';

part 'DrawerContentGroups.dart';

enum _WorkspaceBindingChoice { existing, folder, create }

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
    this.onCollapseSidebar,
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
  final VoidCallback? onCollapseSidebar;
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
  final Set<String> _collapsedGroups = <String>{};
  Set<SidebarDirectory> _pinnedDirectories = const <SidebarDirectory>{};
  bool _searchExpanded = false;
  String? _errorMessage;
  bool _groupMutationPending = false;
  List<core_proxy.Workspace> _workspaces = const <core_proxy.Workspace>[];
  int _workspaceLoadGeneration = 0;
  StreamSubscription<List<core_proxy.Workspace>>? _workspacesSubscription;
  StreamSubscription<Set<SidebarDirectory>>? _directoryPinsSubscription;

  /// Uses the actual receiving bridge for native conversation operations.
  GeneratedCoreProxyClients get _clients =>
      GeneratedCoreProxyClients(widget.bridge);

  /// Returns the shared canonical chat runtime without creating business managers.
  GeneratedChatRuntimeHolderMainCoreProxy get _chatCoreProxy =>
      _clients.chatRuntimeHolderMain;

  SidebarDockPreferences get _directoryPreferences =>
      SidebarDockPreferences(clients: _clients);

  /// Observes native workspace search independently of plugin-owned tab state.
  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
    unawaited(_loadWorkspaces());
    _workspacesSubscription = _clients.repositoryWorkspacePreferenceStore
        .allWorkspacesFlow()
        .listen(
          (workspaces) {
            if (!mounted) return;
            setState(() {
              _workspaces = List<core_proxy.Workspace>.unmodifiable(workspaces);
            });
          },
          onError: (Object error) {
            debugPrint('Failed to watch workspaces: $error');
          },
        );
    _directoryPinsSubscription = _directoryPreferences
        .watchDirectoryPins()
        .listen(
          (directories) {
            if (!mounted) return;
            setState(() => _pinnedDirectories = directories);
          },
          onError: (Object error) {
            debugPrint('Failed to watch sidebar directory pins: $error');
            if (mounted) setState(() => _errorMessage = error.toString());
          },
        );
  }

  /// Clears local errors only after fresh canonical drawer state arrives.
  @override
  void didUpdateWidget(covariant DrawerContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.errorMessage != widget.errorMessage) _errorMessage = null;
    if (_workspaceMembershipFingerprint(oldWidget.histories) !=
        _workspaceMembershipFingerprint(widget.histories)) {
      unawaited(_loadWorkspaces());
    }
  }

  String _workspaceMembershipFingerprint(
    List<core_proxy.ChatHistoryListItem> histories,
  ) => histories
      .map(
        (history) =>
            '${history.id}:${history.workspaceId ?? ''}:${history.workspaceName ?? ''}',
      )
      .join('|');

  Future<void> _loadWorkspaces() async {
    final generation = ++_workspaceLoadGeneration;
    try {
      final workspaces = await _clients.servicesWorkspaceService
          .listWorkspaces();
      if (!mounted || generation != _workspaceLoadGeneration) return;
      setState(() {
        _workspaces = List<core_proxy.Workspace>.unmodifiable(workspaces);
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load workspaces: $error\n$stackTrace');
      if (!mounted || generation != _workspaceLoadGeneration) return;
      setState(() => _errorMessage = error.toString());
    }
  }

  /// Releases native controllers without modifying persisted plugin tab selection.
  @override
  void dispose() {
    unawaited(_workspacesSubscription?.cancel());
    unawaited(_directoryPinsSubscription?.cancel());
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

  Future<void> _showBindWorkspaceDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    final chatId = history.id;
    if (chatId.trim().isEmpty) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('对话 ID 无效')));
      return;
    }
    final choice = await showDialog<_WorkspaceBindingChoice>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('绑定对话到项目'),
        children: <Widget>[
          if (_workspaces.any(
            (workspace) =>
                workspace.id.isNotEmpty && workspace.id != history.workspaceId,
          ))
            SimpleDialogOption(
              onPressed: () =>
                  Navigator.of(context).pop(_WorkspaceBindingChoice.existing),
              child: const ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Icon(Icons.folder_special_outlined),
                title: Text('选择已有项目'),
                subtitle: Text('把当前对话移动到已有项目'),
              ),
            ),
          SimpleDialogOption(
            onPressed: () =>
                Navigator.of(context).pop(_WorkspaceBindingChoice.folder),
            child: const ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.folder_open_outlined),
              title: Text('绑定本地文件夹'),
              subtitle: Text('无项目时创建项目，有项目时追加文件夹'),
            ),
          ),
          SimpleDialogOption(
            onPressed: () =>
                Navigator.of(context).pop(_WorkspaceBindingChoice.create),
            child: const ListTile(
              contentPadding: EdgeInsets.zero,
              leading: Icon(Icons.create_new_folder_outlined),
              title: Text('新建空白项目'),
              subtitle: Text('创建内部项目并移动当前对话'),
            ),
          ),
        ],
      ),
    );
    if (!mounted || choice == null) return;

    switch (choice) {
      case _WorkspaceBindingChoice.existing:
        await _showMoveConversationToWorkspaceDialog(history);
      case _WorkspaceBindingChoice.folder:
        final source = await OperitFolderAccess.pickWorkspaceDirectory();
        if (!mounted || source == null) return;
        await _runGroupMutation(
          () => _chatCoreProxy.bindChatToWorkspace(
            chatId: chatId,
            workspace: source.encode(),
          ),
        );
      case _WorkspaceBindingChoice.create:
        final name = await _groupNameDialog('新建项目', '', labelText: '项目名称');
        if (!mounted || name == null) return;
        await _runGroupMutation(() async {
          await _chatCoreProxy.createAndBindWorkspace(
            chatId: chatId,
            name: name,
          );
        });
    }
  }

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
      await _chatCoreProxy.switchChat(chatId: chatId);
      ClientLogger.v(
        'chat_switch.command_completed chatId=$chatId elapsedMs=${switchStartedAt.elapsedMilliseconds}',
        tag: 'ChatSwitchTrace',
        verboseLevel: VerboseLevel.level2,
      );
    } catch (error, stackTrace) {
      ChatSelectionTransition.complete(chatId);
      debugPrint('Failed to switch chat: $error\n$stackTrace');
      if (mounted) setState(() => _errorMessage = error.toString());
      rethrow;
    }
  }

  /// Creates in an explicit workspace or inherits the native tab's selected binding.
  Future<void> _createConversation({
    String? workspaceId,
    bool inheritWorkspace = false,
  }) async {
    final chatCore = _chatCoreProxy;
    final onActivated = widget.onConversationActivated;
    newChatIntroArmed.value = true;
    if (mounted) setState(() => _errorMessage = null);
    try {
      if (inheritWorkspace) {
        workspaceId = await sidebarNewChatWorkspaceId(
          clients: _clients,
          histories: widget.histories,
          currentChatId: widget.currentChatId,
        );
      }
      await chatCore.createNewChat(
        setAsCurrentChat: true,
        sourceChatId: null,
        parentChatId: null,
        workspaceId: workspaceId,
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

  /// Confirms a swipe deletion before calling the canonical conversation API.
  Future<void> _showDeleteConversationDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
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

  /// Shares the host's current theme and desktop launcher with all sidebar tabs.
  Future<void> _openConversationInWindow(
    core_proxy.ChatHistoryListItem history,
  ) => DetachedChatWindowLauncher.openChat(
    chatId: history.id,
    title: history.title,
    themePreferenceSnapshot: OperitTheme.of(context).themePreferenceSnapshot,
  );

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
          canMoveToWorkspace: _workspaces.any(
            (workspace) => workspace.id != history.workspaceId,
          ),
        );
      },
    );
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case ConversationAction.searchMessages:
        await _openMessageLookup(history);
      case ConversationAction.viewFavorites:
        await _openMessageLookup(history, favoritesOnly: true);
      case ConversationAction.openInWindow:
        await _openConversationInWindow(history);
      case ConversationAction.rename:
        await _showRenameConversationDialog(history);
      case ConversationAction.moveUp:
        await _moveConversationRelative(history, -1);
      case ConversationAction.moveDown:
        await _moveConversationRelative(history, 1);
      case ConversationAction.bindWorkspace:
        await _showBindWorkspaceDialog(history);
      case ConversationAction.moveToWorkspace:
        await _showMoveConversationToWorkspaceDialog(history);
      case ConversationAction.togglePinned:
        await _updateConversationPinned(history);
      case ConversationAction.toggleLocked:
        await _updateConversationLocked(history);
      case ConversationAction.unbindWorkspace:
        await _showUnbindConversationWorkspaceDialog(history);
      case ConversationAction.delete:
        await _deleteConversation(history);
    }
  }

  Future<void> _openMessageLookup(
    core_proxy.ChatHistoryListItem history, {
    bool favoritesOnly = false,
  }) async {
    try {
      await _activateChat(history.id);
    } catch (_) {
      // Activation already exposes its failure in the sidebar.
      return;
    }
    ChatMessageLookupRequests.open(history.id, favoritesOnly: favoritesOnly);
  }

  Future<void> _showMoveConversationToWorkspaceDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    final workspaceId = await showDialog<String>(
      context: context,
      useRootNavigator: true,
      builder: (context) => WorkspacePickerDialog(
        workspaces: _workspaces,
        currentWorkspaceId: history.workspaceId,
      ),
    );
    if (!mounted || workspaceId == null) return;
    await _runGroupMutation(
      () => _chatCoreProxy.moveChatToWorkspace(
        chatId: history.id,
        workspaceId: workspaceId,
      ),
    );
  }

  Future<void> _deleteWorkspace(_WorkspaceSection section) async {
    final workspaceId = section.workspaceId;
    if (workspaceId == null || _groupMutationPending) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => DeleteWorkspaceDialog(name: section.label),
    );
    if (!mounted || confirmed != true) return;
    await _runGroupMutation(() async {
      await _chatCoreProxy.deleteWorkspace(workspaceId: workspaceId);
      await _directoryPreferences.setDirectoryPinned(
        SidebarDirectory.workspace(workspaceId),
        false,
      );
      if (!mounted) return;
      setState(() {
        _collapsedWorkspaces.remove(section.key);
        _rememberedCollapsedWorkspaces.remove(section.key);
        _expandedWorkspaces.remove(section.key);
      });
      await _loadWorkspaces();
    });
  }

  /// Lets a conversation detach an exact workspace binding even when its record is missing.
  Future<void> _showUnbindConversationWorkspaceDialog(
    core_proxy.ChatHistoryListItem history,
  ) async {
    await showDialog<void>(
      context: context,
      useRootNavigator: true,
      builder: (dialogContext) => WorkspaceUnbindDialog(
        onUnbindWorkspace: () async {
          final currentHistory = widget.histories
              .where((chat) => chat.id == history.id)
              .single;
          if (currentHistory.workspaceId != history.workspaceId) {
            throw StateError(
              AppLocalizations.of(dialogContext)!.workspaceUnbindChanged,
            );
          }
          await _chatCoreProxy.unbindChatFromWorkspace(chatId: history.id);
        },
      ),
    );
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

  /// Groups unbound conversations together and preserves bound workspace identities.
  String _workspaceKey(core_proxy.ChatHistoryListItem history) {
    final id = history.workspaceId?.trim();
    return id == null || id.isEmpty ? 'workspace:unbound' : 'workspace:$id';
  }

  /// Distinguishes missing workspace records from conversations without a binding.
  String _workspaceLabel(core_proxy.ChatHistoryListItem history) {
    final id = history.workspaceId?.trim();
    if (id == null || id.isEmpty) {
      return '未绑定项目';
    }
    final name = history.workspaceName?.trim();
    if (name == null || name.isEmpty) {
      return '项目记录缺失';
    }
    return name;
  }

  bool _matchesHistory(core_proxy.ChatHistoryListItem history, String query) =>
      query.isEmpty ||
      history.title.toLowerCase().contains(query) ||
      (history.group ?? '未分组').toLowerCase().contains(query) ||
      _workspaceLabel(history).toLowerCase().contains(query);

  /// Projects persisted workspaces first, then places canonical chats inside them.
  List<_WorkspaceSection> _workspaceSections({bool filter = true}) {
    final query = _searchController.text.trim().toLowerCase();
    final historiesByWorkspace =
        <String?, List<core_proxy.ChatHistoryListItem>>{};
    for (final history in widget.histories) {
      final workspaceId = history.workspaceId?.trim();
      historiesByWorkspace
          .putIfAbsent(
            workspaceId == null || workspaceId.isEmpty ? null : workspaceId,
            () => <core_proxy.ChatHistoryListItem>[],
          )
          .add(history);
    }

    final sections = <_WorkspaceSection>[];
    final unbound =
        historiesByWorkspace[null] ?? const <core_proxy.ChatHistoryListItem>[];
    if (unbound.any((history) => !filter || _matchesHistory(history, query))) {
      final section = _WorkspaceSection(
        key: 'workspace:unbound',
        label: '未绑定项目',
        canCreate: true,
      );
      section.histories.addAll(
        unbound.where((history) => !filter || _matchesHistory(history, query)),
      );
      sections.add(section);
    }

    final knownWorkspaceIds = <String>{};
    for (final workspace in _workspaces) {
      final workspaceId = workspace.id.trim();
      if (workspaceId.isEmpty) continue;
      knownWorkspaceIds.add(workspaceId);
      final histories =
          historiesByWorkspace[workspaceId] ??
          const <core_proxy.ChatHistoryListItem>[];
      final workspaceMatches =
          !filter ||
          query.isEmpty ||
          workspace.name.toLowerCase().contains(query) ||
          workspace.folders.any(
            (folder) => folder.path.toLowerCase().contains(query),
          );
      final visibleHistories = workspaceMatches
          ? histories
          : histories.where((history) => _matchesHistory(history, query));
      if (!workspaceMatches && visibleHistories.isEmpty) continue;
      final section = _WorkspaceSection(
        key: 'workspace:$workspaceId',
        label: workspace.name,
        workspaceId: workspaceId,
        subtitle: workspace.folders.isEmpty
            ? null
            : workspace.folders.first.path,
        canCreate: true,
      );
      section.histories.addAll(visibleHistories);
      sections.add(section);
    }

    for (final entry in historiesByWorkspace.entries) {
      final workspaceId = entry.key;
      if (workspaceId == null || knownWorkspaceIds.contains(workspaceId)) {
        continue;
      }
      final visibleHistories = entry.value.where(
        (history) => !filter || _matchesHistory(history, query),
      );
      if (visibleHistories.isEmpty) continue;
      final first = entry.value.first;
      final section = _WorkspaceSection(
        key: _workspaceKey(first),
        label: _workspaceLabel(first),
        workspaceId: workspaceId,
        canCreate: false,
      );
      section.histories.addAll(visibleHistories);
      sections.add(section);
    }
    return [
      ...sections.where(
        (section) => _pinnedDirectories.contains(
          SidebarDirectory.workspace(section.workspaceId),
        ),
      ),
      ...sections.where(
        (section) => !_pinnedDirectories.contains(
          SidebarDirectory.workspace(section.workspaceId),
        ),
      ),
    ];
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

  bool get _isChatRouteSelected =>
      widget.selectedRouteId ==
      ScreenRouteRegistry.routeIdOf(ScreenRouteRegistry.aiChat);

  /// Builds a native row with the original pin, lock, title, delete and detached-window actions.
  Widget _historyRow(core_proxy.ChatHistoryListItem history) =>
      ConversationDrawerItem(
        key: ValueKey('workspace-chat:${history.id}'),
        history: history,
        title: history.title,
        selected: _isChatRouteSelected && widget.currentChatId == history.id,
        isRunning: widget.activeStreamingChatIds.contains(history.id),
        appearance: widget.appearance,
        nested: true,
        workspaceStyle: true,
        onClick: () => unawaited(_switchConversation(history)),
        onSearchMessages: () => unawaited(_openMessageLookup(history)),
        onViewFavorites: () =>
            unawaited(_openMessageLookup(history, favoritesOnly: true)),
        onRename: () => unawaited(_showRenameConversationDialog(history)),
        onBindWorkspace: () => unawaited(_showBindWorkspaceDialog(history)),
        onTogglePinned: () => unawaited(_updateConversationPinned(history)),
        onToggleLocked: () => unawaited(_updateConversationLocked(history)),
        onDelete: () => unawaited(_deleteConversation(history)),
        onSwipeDelete: () => unawaited(_showDeleteConversationDialog(history)),
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

  Widget _sidebarHeader(Widget tabs) {
    final onCollapse = widget.onCollapseSidebar;
    final desktopHeader = useTabletLayoutForContext(context);
    final topPadding = MediaQuery.paddingOf(context).top;
    final header = Padding(
      padding: EdgeInsets.only(top: desktopHeader ? topPadding : 26, right: 12),
      child: Row(
        children: <Widget>[
          Expanded(
            child: SidebarInfoCard(
              brandName: 'Operit',
              appearance: widget.appearance,
              trailing: tabs,
            ),
          ),
          if (onCollapse != null) ...<Widget>[
            const SizedBox(width: 4),
            IconButton(
              tooltip: '收起侧边栏',
              onPressed: onCollapse,
              icon: Icon(
                Icons.chevron_left,
                size: 20,
                color: widget.appearance.itemColor.withValues(alpha: 0.82),
              ),
              visualDensity: VisualDensity.compact,
              padding: EdgeInsets.zero,
              constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
            ),
          ],
        ],
      ),
    );
    return desktopHeader
        ? SizedBox(
            height: topPadding + ThemeTopBarChrome.toolbarHeight,
            child: header,
          )
        : header;
  }

  /// Adds plugin navigation to the native workspace's existing scrolling surface.
  List<Widget> _pluginNavigationSlivers() => <Widget>[
    if (widget.pluginEntries.isNotEmpty)
      SliverToBoxAdapter(child: _pluginNavigationContent()),
  ];

  /// Sizes plugin navigation to its content for the owning sidebar scroll.
  Widget _pluginNavigationContent() => Column(
    mainAxisSize: MainAxisSize.min,
    children: <Widget>[
      const SizedBox(height: 10),
      for (final entry in widget.pluginEntries)
        PluginNavigationDrawerItem(
          entry: entry,
          selected: widget.selectedRouteId == entry.routeId,
          appearance: widget.appearance,
          onClick: () => widget.onNavigationEntrySelected(entry),
        ),
      SidebarDockEndDropTarget(
        controller:
            MediaQuery.sizeOf(context).width >= navigationTabletBreakpoint
            ? SidebarDockScope.maybeOf(context)
            : null,
        location: SidebarDockLocation.primary,
        height: 18,
      ),
    ],
  );

  /// Renders collapsible groups and their flat conversation rows within a project.
  Widget _workspaceHistory(_WorkspaceSection section, bool searching) {
    final groups = _nativeGroups(section);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final group in groups) ...<Widget>[
          _GroupSectionHeader(
            key: ValueKey('workspace-group:${group.key}'),
            group: group,
            pinned: _pinnedDirectories.contains(group.directory),
            expanded: !_collapsedGroups.contains(group.key),
            appearance: widget.appearance,
            canCreate: section.canCreate,
            onToggleExpanded: () => setState(() {
              if (!_collapsedGroups.remove(group.key)) {
                _collapsedGroups.add(group.key);
              }
            }),
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
          if (!_collapsedGroups.contains(group.key)) ...<Widget>[
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
                          onCreateConversation: () => unawaited(
                            _createConversation(inheritWorkspace: true),
                          ),
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
                  subtitle: section.subtitle,
                  pinned: _pinnedDirectories.contains(
                    SidebarDirectory.workspace(section.workspaceId),
                  ),
                  expanded: !_collapsedWorkspaces.contains(section.key),
                  appearance: widget.appearance,
                  canCreate: section.canCreate,
                  onDelete: section.workspaceId != null && section.canCreate
                      ? () => unawaited(_deleteWorkspace(section))
                      : null,
                  onToggleExpanded: () => _toggleWorkspace(section.key),
                  onCreateConversation: () => unawaited(
                    _createConversation(workspaceId: section.workspaceId),
                  ),
                  onCreateGroup: () =>
                      unawaited(_createWorkspaceGroup(section)),
                  onTogglePinned: () => unawaited(
                    _toggleDirectoryPinned(
                      SidebarDirectory.workspace(section.workspaceId),
                    ),
                  ),
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
            currentChatId: _isChatRouteSelected ? widget.currentChatId : null,
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
                : _pluginNavigationContent(),
            onActivateChat: _activateChat,
            onOpenChatInWindow: operitSupportsDesktopMultiWindow
                ? _openConversationInWindow
                : null,
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
  const _WorkspaceCreateBar({required this.onCreateConversation});
  final VoidCallback onCreateConversation;

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
                borderRadius: BorderRadius.circular(17),
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
                      Flexible(
                        child: Text(
                          '新建对话',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(
                            fontSize: 13,
                            fontWeight: FontWeight.w600,
                            color: contentColor,
                            letterSpacing: -0.1,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _WorkspaceSection {
  /// Retains complete native conversation rows for grouping, preview and search.
  _WorkspaceSection({
    required this.key,
    required this.label,
    this.workspaceId,
    this.subtitle,
    required this.canCreate,
  });
  final String key;
  final String label;
  final String? workspaceId;
  final String? subtitle;
  final bool canCreate;
  final List<core_proxy.ChatHistoryListItem> histories =
      <core_proxy.ChatHistoryListItem>[];
}

enum _WorkspaceQuickAction { createChat, createGroup, togglePinned, delete }

class _WorkspaceHeader extends StatelessWidget {
  const _WorkspaceHeader({
    super.key,
    required this.label,
    this.subtitle,
    required this.pinned,
    required this.expanded,
    required this.appearance,
    required this.canCreate,
    required this.onToggleExpanded,
    required this.onCreateConversation,
    required this.onCreateGroup,
    required this.onTogglePinned,
    this.onDelete,
  });

  final String label;
  final String? subtitle;
  final bool pinned;
  final bool expanded;
  final NavigationDrawerAppearance appearance;
  final bool canCreate;
  final VoidCallback onToggleExpanded;
  final VoidCallback onCreateConversation;
  final VoidCallback onCreateGroup;
  final VoidCallback onTogglePinned;
  final VoidCallback? onDelete;

  void _select(_WorkspaceQuickAction action) {
    switch (action) {
      case _WorkspaceQuickAction.createChat:
        onCreateConversation();
      case _WorkspaceQuickAction.createGroup:
        onCreateGroup();
      case _WorkspaceQuickAction.togglePinned:
        onTogglePinned();
      case _WorkspaceQuickAction.delete:
        onDelete?.call();
    }
  }

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
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelLarge?.copyWith(
                      color: appearance.titleColor,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  if (subtitle != null && subtitle!.trim().isNotEmpty)
                    Text(
                      subtitle!,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: appearance.itemColor.withValues(alpha: 0.56),
                        fontSize: 10,
                      ),
                    ),
                ],
              ),
            ),
            if (pinned) ...<Widget>[
              const SizedBox(width: 4),
              Icon(
                Icons.push_pin_rounded,
                size: 13,
                color: appearance.itemColor.withValues(alpha: 0.65),
              ),
            ],
            const SizedBox(width: 4),
            SidebarMoreMenuButton<_WorkspaceQuickAction>(
              compact: true,
              tooltip: '项目操作',
              color: appearance.itemColor.withValues(alpha: 0.78),
              onSelected: _select,
              actions: <SidebarMenuAction<_WorkspaceQuickAction>>[
                SidebarMenuAction(
                  value: _WorkspaceQuickAction.createChat,
                  icon: Icons.add_comment_outlined,
                  label: '新建对话',
                  enabled: canCreate,
                ),
                SidebarMenuAction(
                  value: _WorkspaceQuickAction.createGroup,
                  icon: Icons.create_new_folder_outlined,
                  label: '新建分组',
                  enabled: canCreate,
                ),
                SidebarMenuAction(
                  value: _WorkspaceQuickAction.togglePinned,
                  icon: pinned
                      ? Icons.push_pin_outlined
                      : Icons.push_pin_rounded,
                  label: pinned ? '取消置顶' : '置顶',
                ),
                if (onDelete != null)
                  const SidebarMenuAction(
                    value: _WorkspaceQuickAction.delete,
                    icon: Icons.delete_outline_rounded,
                    label: '删除工作区',
                    dividerBefore: true,
                    danger: true,
                  ),
              ],
            ),
            const SizedBox(width: 4),
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
  Widget build(BuildContext context) => ThemeBottomThemeToggleButton(
    appearance: appearance,
    darkThemeActive: darkThemeActive,
    onToggle: onToggle,
  );
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
    return ThemeDrawerToolbarIconButton(
      icon: icon,
      tooltip: tooltip,
      appearance: appearance,
      onClick: onClick,
      active: active,
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
        start: workspaceStyle ? 50 : 56,
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
          padding: EdgeInsets.zero,
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
