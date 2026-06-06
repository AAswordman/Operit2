// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/link/CoreLinkProtocol.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../features/chat/viewmodel/ChatSwitchRenderCoordinator.dart';
import '../navigation/AppNavigationModels.dart';
import '../screens/ScreenRouteRegistry.dart';
import '../../theme/OperitTheme.dart';
import 'CollapsedDrawerContent.dart';
import 'DrawerContentDialogs.dart';
import 'NavigationDrawerAppearance.dart';

class DrawerContent extends StatefulWidget {
  const DrawerContent({
    super.key,
    required this.navigationEntries,
    required this.selectedRouteId,
    required this.appearance,
    required this.onNavigationEntrySelected,
    required this.onConversationActivated,
    this.bridge = const ProxyCoreRuntimeBridge(),
  });

  final List<NavigationEntrySpec> navigationEntries;
  final String selectedRouteId;
  final NavigationDrawerAppearance appearance;
  final ValueChanged<NavigationEntrySpec> onNavigationEntrySelected;
  final VoidCallback onConversationActivated;
  final OperitRuntimeBridge bridge;

  @override
  State<DrawerContent> createState() => _DrawerContentState();
}

class _DrawerContentState extends State<DrawerContent> {
  static const int _collapsedHistoryLimit = 4;
  static const double _contentEndPadding = 12;
  static final Set<String> _rememberedCollapsedCharacterSections = <String>{};
  static final Set<String> _rememberedCollapsedGroupSections = <String>{};

  final ScrollController _historyScrollController = ScrollController();
  final TextEditingController _searchController = TextEditingController();
  final List<core_proxy.ChatHistory> _histories = <core_proxy.ChatHistory>[];
  final Map<String, String> _characterGroupNamesById = <String, String>{};
  final Set<String> _collapsedCharacterSections = Set<String>.of(
    _rememberedCollapsedCharacterSections,
  );
  final Set<String> _collapsedGroupSections = Set<String>.of(
    _rememberedCollapsedGroupSections,
  );
  StreamSubscription<List<core_proxy.ChatHistory>>? _historiesSubscription;
  StreamSubscription<String?>? _currentChatSubscription;
  StreamSubscription<List<core_proxy.CharacterGroupCard>>?
  _characterGroupsSubscription;
  String? _currentChatId;
  String? _errorMessage;
  bool _loading = true;
  int _historyRenderLimit = _collapsedHistoryLimit;
  bool _searchExpanded = false;

  GeneratedChatRuntimeHolderMainCoreProxy get _chatCoreProxy =>
      GeneratedCoreProxyClients(widget.bridge).chatRuntimeHolderMain;

  GeneratedPreferencesCharacterGroupCardManagerCoreProxy
  get _characterGroupCoreProxy => GeneratedCoreProxyClients(
    widget.bridge,
  ).preferencesCharacterGroupCardManager;

  String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
    _loadConversations();
    _watchConversations();
    _watchCharacterGroups();
  }

  @override
  void dispose() {
    _historiesSubscription?.cancel();
    _currentChatSubscription?.cancel();
    _characterGroupsSubscription?.cancel();
    _historyScrollController.dispose();
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _loadConversations() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      await _characterGroupCoreProxy.initializeIfNeeded();
      final results = await Future.wait<Object?>(<Future<Object?>>[
        _chatCoreProxy.chatHistoriesFlowSnapshot(),
        _chatCoreProxy.currentChatIdFlowSnapshot(),
        _characterGroupCoreProxy.allCharacterGroupCardsFlowSnapshot(),
      ]);
      final histories = results[0] as List<core_proxy.ChatHistory>;
      final currentChatId = results[1] as String?;
      final characterGroups = results[2] as List<core_proxy.CharacterGroupCard>;
      if (!mounted) {
        return;
      }
      setState(() {
        _histories
          ..clear()
          ..addAll(histories);
        _characterGroupNamesById
          ..clear()
          ..addAll(_characterGroupNameMap(characterGroups));
        _currentChatId = currentChatId;
        _loading = false;
      });
    } catch (error, stackTrace) {
      debugPrint('Failed to load chat histories: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
        _loading = false;
      });
    }
  }

  void _watchConversations() {
    _historiesSubscription?.cancel();
    _historiesSubscription = _chatCoreProxy.chatHistoriesFlowChanges().listen(
      (histories) {
        if (!mounted) {
          return;
        }
        setState(() {
          _histories
            ..clear()
            ..addAll(histories);
          _loading = false;
          _errorMessage = null;
        });
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch chat histories: $error\n$stackTrace');
        if (!mounted) {
          return;
        }
        setState(() {
          _errorMessage = error.toString();
          _loading = false;
        });
      },
    );

    _currentChatSubscription?.cancel();
    _currentChatSubscription = _chatCoreProxy.currentChatIdFlowChanges().listen(
      (chatId) {
        if (!mounted) {
          return;
        }
        setState(() {
          _currentChatId = chatId;
        });
      },
      onError: (Object error, StackTrace stackTrace) {
        debugPrint('Failed to watch current chat id: $error\n$stackTrace');
        if (!mounted) {
          return;
        }
        setState(() {
          _errorMessage = error.toString();
        });
      },
    );
  }

  void _watchCharacterGroups() {
    _characterGroupsSubscription?.cancel();
    _characterGroupsSubscription = _characterGroupCoreProxy
        .allCharacterGroupCardsFlowChanges()
        .listen(
          (groups) {
            if (!mounted) {
              return;
            }
            setState(() {
              _characterGroupNamesById
                ..clear()
                ..addAll(_characterGroupNameMap(groups));
            });
          },
          onError: (Object error, StackTrace stackTrace) {
            debugPrint('Failed to watch character groups: $error\n$stackTrace');
            if (!mounted) {
              return;
            }
            setState(() {
              _errorMessage = error.toString();
            });
          },
        );
  }

  Map<String, String> _characterGroupNameMap(
    List<core_proxy.CharacterGroupCard> groups,
  ) {
    return <String, String>{
      for (final group in groups)
        if (group.id.trim().isNotEmpty && group.name.trim().isNotEmpty)
          group.id.trim(): group.name.trim(),
    };
  }

  void _onSearchChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  void _openPackageManager() {
    for (final entry in widget.navigationEntries) {
      if (entry.entryId == 'main.package_manager') {
        widget.onNavigationEntrySelected(entry);
        return;
      }
    }
    throw StateError('Unknown navigation entry: main.package_manager');
  }

  void _openSettings() {
    for (final entry in widget.navigationEntries) {
      if (entry.entryId == 'main.settings') {
        widget.onNavigationEntrySelected(entry);
        return;
      }
    }
    throw StateError('Unknown navigation entry: main.settings');
  }

  void _toggleSearchExpanded() {
    setState(() {
      _searchExpanded = !_searchExpanded;
    });
  }

  Future<void> _createConversation() async {
    setState(() {
      _errorMessage = null;
    });
    try {
      await _chatCoreProxy.createNewChat(
        characterCardName: null,
        group: null,
        inheritGroupFromCurrent: true,
        setAsCurrentChat: true,
        characterGroupId: null,
      );
      widget.onConversationActivated();
    } catch (error, stackTrace) {
      debugPrint('Failed to create chat: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _showCreateGroupDialog() async {
    final groupName = await showDialog<String>(
      context: context,
      builder: (context) {
        return const CreateGroupDialog();
      },
    );
    final normalizedGroupName = groupName?.trim();
    if (normalizedGroupName == null || normalizedGroupName.isEmpty) {
      return;
    }
    await _createGroup(normalizedGroupName);
  }

  Future<void> _createGroup(String groupName) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      final binding = await _activePromptBindingForCreate();
      await widget.bridge.call(
        CoreCallRequest(
          requestId: _requestId(),
          targetPath: CoreObjectPath.parse('chatRuntimeHolder.main'),
          methodName: 'createGroup',
          args: <String, Object?>{
            'groupName': groupName,
            'characterCardName': binding.characterCardName,
            'characterGroupId': binding.characterGroupId,
          },
        ),
      );
      widget.onConversationActivated();
    } catch (error, stackTrace) {
      debugPrint('Failed to create group: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  Future<_ChatBindingForCreate> _activePromptBindingForCreate() async {
    final activePrompt = await widget.bridge.call(
      CoreCallRequest(
        requestId: _requestId(),
        targetPath: CoreObjectPath.parse('preferences.activePromptManager'),
        methodName: 'getActivePrompt',
        args: const <String, Object?>{},
      ),
    );
    final prompt = activePrompt as Map<String, Object?>;
    final characterGroup = prompt['CharacterGroup'] as Map<String, Object?>?;
    if (characterGroup != null) {
      return _ChatBindingForCreate(
        characterCardName: null,
        characterGroupId: characterGroup['id'] as String,
      );
    }
    final characterCard = prompt['CharacterCard'] as Map<String, Object?>?;
    if (characterCard != null) {
      final id = characterCard['id'] as String;
      final card = await widget.bridge.call(
        CoreCallRequest(
          requestId: _requestId(),
          targetPath: CoreObjectPath.parse('preferences.characterCardManager'),
          methodName: 'getCharacterCard',
          args: <String, Object?>{'id': id},
        ),
      );
      return _ChatBindingForCreate(
        characterCardName: (card as Map<String, Object?>)['name'] as String,
        characterGroupId: null,
      );
    }
    throw StateError('Unknown active prompt payload: $prompt');
  }

  Future<void> _switchConversation(core_proxy.ChatHistory history) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      ChatSwitchRenderCoordinator.prepareForChat(history.id);
      widget.onConversationActivated();
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted) {
        return;
      }
      await _chatCoreProxy.switchChat(chatId: history.id);
    } catch (error, stackTrace) {
      ChatSwitchRenderCoordinator.clear();
      debugPrint('Failed to switch chat: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  Future<void> _showRenameConversationDialog(
    core_proxy.ChatHistory history,
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

  Future<void> _showDeleteConversationDialog(
    core_proxy.ChatHistory history,
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

  Future<void> _showConversationActionDialog(
    core_proxy.ChatHistory history,
  ) async {
    final index = _histories.indexWhere((item) => item.id == history.id);
    final action = await showDialog<ConversationAction>(
      context: context,
      useRootNavigator: true,
      builder: (context) {
        return ConversationActionDialog(
          history: history,
          canMoveUp: index > 0,
          canMoveDown: index >= 0 && index < _histories.length - 1,
        );
      },
    );
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
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

  Future<void> _deleteConversation(core_proxy.ChatHistory history) async {
    setState(() {
      _errorMessage = null;
    });
    try {
      await _chatCoreProxy.deleteChatHistory(chatId: history.id);
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

  Future<void> _updateConversationTitle(
    core_proxy.ChatHistory history,
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

  Future<void> _updateConversationPinned(core_proxy.ChatHistory history) async {
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

  Future<void> _updateConversationLocked(core_proxy.ChatHistory history) async {
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

  Future<void> _moveConversationRelative(
    core_proxy.ChatHistory history,
    int delta,
  ) async {
    final currentIndex = _histories.indexWhere((item) => item.id == history.id);
    final targetIndex = currentIndex + delta;
    if (currentIndex < 0 ||
        targetIndex < 0 ||
        targetIndex >= _histories.length) {
      return;
    }
    final reordered = List<core_proxy.ChatHistory>.of(_histories);
    final moved = reordered.removeAt(currentIndex);
    reordered.insert(targetIndex, moved);
    await _updateConversationOrder(
      reordered,
      moved,
      moved.group,
      optimistic: true,
    );
  }

  Future<void> _moveConversationTo(
    core_proxy.ChatHistory moved,
    core_proxy.ChatHistory target,
  ) async {
    if (moved.id == target.id) {
      return;
    }
    final reordered = List<core_proxy.ChatHistory>.of(_histories);
    final fromIndex = reordered.indexWhere((item) => item.id == moved.id);
    final toIndex = reordered.indexWhere((item) => item.id == target.id);
    if (fromIndex < 0 || toIndex < 0) {
      return;
    }
    final removed = reordered.removeAt(fromIndex);
    final insertIndex = toIndex > reordered.length ? reordered.length : toIndex;
    reordered.insert(insertIndex, removed);
    await _updateConversationOrder(
      reordered,
      removed,
      target.group,
      optimistic: true,
    );
  }

  Future<void> _updateConversationOrder(
    List<core_proxy.ChatHistory> reordered,
    core_proxy.ChatHistory moved,
    String? targetGroup, {
    required bool optimistic,
  }) async {
    final orderedJson = <Map<String, Object?>>[];
    for (var index = 0; index < reordered.length; index += 1) {
      final json = reordered[index].toJson();
      json['messages'] = const <Object?>[];
      json['displayOrder'] = index;
      if (json['id'] == moved.id) {
        json['group'] = targetGroup;
      }
      orderedJson.add(json);
    }
    final updatedHistories = orderedJson
        .map(core_proxy.ChatHistory.fromJson)
        .toList(growable: false);
    final updatedMoved = updatedHistories.firstWhere(
      (history) => history.id == moved.id,
    );
    if (optimistic) {
      setState(() {
        _histories
          ..clear()
          ..addAll(updatedHistories);
      });
    }
    try {
      await _chatCoreProxy.updateChatOrderAndGroup(
        reorderedHistories: updatedHistories,
        movedItem: updatedMoved,
        targetGroup: targetGroup,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to update chat order: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _errorMessage = error.toString();
      });
    }
  }

  List<core_proxy.ChatHistory> get _visibleHistories {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return List<core_proxy.ChatHistory>.unmodifiable(_histories);
    }
    return _histories
        .where((history) => _historyMatchesQuery(history, query))
        .toList(growable: false);
  }

  bool _historyMatchesQuery(core_proxy.ChatHistory history, String query) {
    return history.title.toLowerCase().contains(query) ||
        _bindingLabel(history).toLowerCase().contains(query) ||
        _groupLabel(history).toLowerCase().contains(query);
  }

  List<_CharacterHistorySection> _buildCharacterSections(
    List<core_proxy.ChatHistory> histories,
  ) {
    final sections = <_CharacterHistorySection>[];
    final sectionIndexes = <String, int>{};
    for (final history in histories) {
      final sectionKey = _characterSectionKey(history);
      final sectionIndex = sectionIndexes[sectionKey];
      final groupKey = _groupSectionKey(sectionKey, history);
      final groupLabel = _groupLabel(history);
      if (sectionIndex == null) {
        sectionIndexes[sectionKey] = sections.length;
        sections.add(
          _CharacterHistorySection(
            key: sectionKey,
            label: _bindingLabel(history),
            kind: _bindingKind(history),
            groups: <_HistoryGroupSection>[
              _HistoryGroupSection(
                key: groupKey,
                label: groupLabel,
                histories: <core_proxy.ChatHistory>[history],
              ),
            ],
          ),
        );
        continue;
      }

      final section = sections[sectionIndex];
      final groupIndex = section.groups.indexWhere(
        (group) => group.key == groupKey,
      );
      if (groupIndex == -1) {
        section.groups.add(
          _HistoryGroupSection(
            key: groupKey,
            label: groupLabel,
            histories: <core_proxy.ChatHistory>[history],
          ),
        );
      } else {
        section.groups[groupIndex].histories.add(history);
      }
    }
    return sections;
  }

  _VisibleHistoryPlan _buildVisibleHistoryPlan(
    List<_CharacterHistorySection> sections,
    int renderLimit,
  ) {
    var remaining = renderLimit;
    var hiddenCount = 0;
    final plannedSections = <_CharacterHistorySection>[];

    for (final section in sections) {
      if (_collapsedCharacterSections.contains(section.key)) {
        plannedSections.add(section);
        continue;
      }

      final plannedGroups = <_HistoryGroupSection>[];
      for (final group in section.groups) {
        if (_collapsedGroupSections.contains(group.key)) {
          plannedGroups.add(group);
          continue;
        }

        final visibleCount = math.min(remaining, group.histories.length);
        final visibleHistories = group.histories
            .take(visibleCount)
            .toList(growable: false);
        hiddenCount += group.histories.length - visibleCount;
        remaining -= visibleCount;
        plannedGroups.add(
          _HistoryGroupSection(
            key: group.key,
            label: group.label,
            histories: visibleHistories,
            historyCount: group.historyCount,
          ),
        );
      }

      plannedSections.add(
        _CharacterHistorySection(
          key: section.key,
          label: section.label,
          kind: section.kind,
          groups: plannedGroups,
        ),
      );
    }

    return _VisibleHistoryPlan(
      sections: plannedSections,
      hiddenCount: hiddenCount,
    );
  }

  String _characterSectionKey(core_proxy.ChatHistory history) {
    final characterGroupId = history.characterGroupId?.trim();
    if (characterGroupId != null && characterGroupId.isNotEmpty) {
      return 'character-group:$characterGroupId';
    }
    final name = history.characterCardName?.trim();
    return name == null || name.isEmpty
        ? 'character:unbound'
        : 'character:$name';
  }

  _HistoryBindingKind _bindingKind(core_proxy.ChatHistory history) {
    final characterGroupId = history.characterGroupId?.trim();
    if (characterGroupId != null && characterGroupId.isNotEmpty) {
      return _HistoryBindingKind.characterGroup;
    }
    final name = history.characterCardName?.trim();
    return name == null || name.isEmpty
        ? _HistoryBindingKind.unbound
        : _HistoryBindingKind.characterCard;
  }

  String _bindingLabel(core_proxy.ChatHistory history) {
    final characterGroupId = history.characterGroupId?.trim();
    if (characterGroupId != null && characterGroupId.isNotEmpty) {
      return _characterGroupNamesById[characterGroupId] ??
          _shortIdentifier(characterGroupId);
    }
    final name = history.characterCardName?.trim();
    return name == null || name.isEmpty ? '未绑定' : name;
  }

  String _groupSectionKey(String sectionKey, core_proxy.ChatHistory history) {
    final group = history.group?.trim();
    final groupPart = group == null || group.isEmpty ? 'ungrouped' : group;
    return 'group::$sectionKey::$groupPart';
  }

  String _groupLabel(core_proxy.ChatHistory history) {
    final group = history.group?.trim();
    return group == null || group.isEmpty ? '未分组' : group;
  }

  void _toggleCharacterSection(String sectionKey) {
    setState(() {
      if (_collapsedCharacterSections.contains(sectionKey)) {
        _collapsedCharacterSections.remove(sectionKey);
      } else {
        _collapsedCharacterSections.add(sectionKey);
      }
      _rememberExpansionState();
    });
  }

  void _toggleGroupSection(String sectionKey) {
    setState(() {
      if (_collapsedGroupSections.contains(sectionKey)) {
        _collapsedGroupSections.remove(sectionKey);
      } else {
        _collapsedGroupSections.add(sectionKey);
      }
      _rememberExpansionState();
    });
  }

  void _showMoreHistories(int hiddenCount) {
    setState(() {
      _historyRenderLimit += hiddenCount;
    });
  }

  void _collapseHistories() {
    setState(() {
      _historyRenderLimit = _collapsedHistoryLimit;
    });
  }

  void _rememberExpansionState() {
    _rememberedCollapsedCharacterSections
      ..clear()
      ..addAll(_collapsedCharacterSections);
    _rememberedCollapsedGroupSections
      ..clear()
      ..addAll(_collapsedGroupSections);
  }

  List<_HistoryListEntry> _buildHistoryEntries(
    List<_CharacterHistorySection> sections,
  ) {
    final entries = <_HistoryListEntry>[];
    for (final section in sections) {
      entries.add(_CharacterHeaderEntry(section));
      if (_collapsedCharacterSections.contains(section.key)) {
        continue;
      }
      for (final group in section.groups) {
        entries.add(_GroupHeaderEntry(group));
        if (_collapsedGroupSections.contains(group.key)) {
          continue;
        }
        for (final history in group.histories) {
          entries.add(_HistoryRowEntry(history));
        }
      }
    }
    return entries;
  }

  @override
  Widget build(BuildContext context) {
    final visibleHistories = _visibleHistories;
    final showInitialLoading =
        _loading && _histories.isEmpty && _errorMessage == null;
    final searching = _searchController.text.trim().isNotEmpty;
    final allCharacterSections = _buildCharacterSections(visibleHistories);
    final historyPlan = searching
        ? _VisibleHistoryPlan(sections: allCharacterSections, hiddenCount: 0)
        : _buildVisibleHistoryPlan(allCharacterSections, _historyRenderLimit);
    final characterSections = historyPlan.sections;
    final hiddenHistoryCount = historyPlan.hiddenCount;
    final historyEntries = _buildHistoryEntries(characterSections);
    final aiChatRouteId = ScreenRouteRegistry.routeIdOf(
      ScreenRouteRegistry.aiChat,
    );
    final packageManagerRouteId = ScreenRouteRegistry.routeIdOf(
      ScreenRouteRegistry.packageManager,
    );
    final settingsRouteId = ScreenRouteRegistry.routeIdOf(
      ScreenRouteRegistry.settings,
    );
    final conversationSelectionEnabled =
        widget.selectedRouteId == aiChatRouteId;
    final themeController = OperitTheme.of(context);
    final darkThemeActive = themeController.isDark(context);
    return Column(
      children: <Widget>[
        Expanded(
          child: Stack(
            children: <Widget>[
              CustomScrollView(
                key: const PageStorageKey<String>('drawer-history-scroll'),
                controller: _historyScrollController,
                primary: false,
                slivers: <Widget>[
                  SliverPadding(
                    padding: const EdgeInsets.fromLTRB(
                      0,
                      30,
                      _contentEndPadding,
                      0,
                    ),
                    sliver: SliverToBoxAdapter(
                      child: SidebarInfoCard(
                        brandName: 'Operit',
                        appearance: widget.appearance,
                      ),
                    ),
                  ),
                  const SliverToBoxAdapter(child: SizedBox(height: 24)),
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsetsDirectional.only(
                        start: 28,
                        end: _contentEndPadding,
                        bottom: 2,
                      ),
                      child: Row(
                        children: <Widget>[
                          Expanded(
                            child: Text(
                              '会话',
                              style: Theme.of(context).textTheme.titleSmall
                                  ?.copyWith(
                                    color: widget.appearance.titleColor
                                        .withValues(alpha: 0.82),
                                    fontWeight: FontWeight.w600,
                                  ),
                            ),
                          ),
                          IconButton(
                            onPressed: () => themeController.toggle(context),
                            visualDensity: VisualDensity.compact,
                            tooltip: darkThemeActive ? '切换白天模式' : '切换黑夜模式',
                            icon: Icon(
                              darkThemeActive
                                  ? Icons.light_mode_outlined
                                  : Icons.dark_mode_outlined,
                              size: 20,
                              color: widget.appearance.itemColor,
                            ),
                          ),
                          IconButton(
                            onPressed: _toggleSearchExpanded,
                            visualDensity: VisualDensity.compact,
                            tooltip: _searchExpanded ? '收起搜索' : '搜索对话',
                            icon: Icon(
                              _searchExpanded ? Icons.search_off : Icons.search,
                              size: 20,
                              color: _searchController.text.trim().isNotEmpty
                                  ? widget.appearance.statusAvailableColor
                                  : widget.appearance.itemColor,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                  const SliverToBoxAdapter(child: SizedBox(height: 6)),
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsetsDirectional.only(
                        start: 12,
                        end: _contentEndPadding,
                        bottom: 8,
                      ),
                      child: NewConversationButton(
                        appearance: widget.appearance,
                        onClick: _createConversation,
                        onCreateGroup: _showCreateGroupDialog,
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
                                end: _contentEndPadding,
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
                  if (_errorMessage != null)
                    SliverToBoxAdapter(
                      child: SidebarStatusText(
                        text: _errorMessage!,
                        appearance: widget.appearance,
                      ),
                    ),
                  SliverList(
                    delegate: SliverChildBuilderDelegate((context, index) {
                      final entry = historyEntries[index];
                      return switch (entry) {
                        _CharacterHeaderEntry(:final section) =>
                          _CharacterSectionHeader(
                            label: section.label,
                            kind: section.kind,
                            count: section.historyCount,
                            expanded: !_collapsedCharacterSections.contains(
                              section.key,
                            ),
                            appearance: widget.appearance,
                            onToggleExpanded: () =>
                                _toggleCharacterSection(section.key),
                          ),
                        _GroupHeaderEntry(:final group) => _GroupSectionHeader(
                          label: group.label,
                          count: group.historyCount,
                          expanded: !_collapsedGroupSections.contains(
                            group.key,
                          ),
                          appearance: widget.appearance,
                          onToggleExpanded: () =>
                              _toggleGroupSection(group.key),
                        ),
                        _HistoryRowEntry(:final history) =>
                          ConversationDrawerItem(
                            history: history,
                            title: history.title,
                            selected:
                                conversationSelectionEnabled &&
                                _currentChatId == history.id,
                            appearance: widget.appearance,
                            nested: true,
                            onClick: () => _switchConversation(history),
                            onRename: () {
                              _showRenameConversationDialog(history);
                            },
                            onDelete: () {
                              _showDeleteConversationDialog(history);
                            },
                            onLongPress: () {
                              _showConversationActionDialog(history);
                            },
                            onMoveTo: (moved) =>
                                _moveConversationTo(moved, history),
                          ),
                      };
                    }, childCount: historyEntries.length),
                  ),
                  if (!searching && hiddenHistoryCount > 0)
                    SliverToBoxAdapter(
                      child: _HistoryLimitButton(
                        icon: Icons.expand_more,
                        label: '展开更多 $hiddenHistoryCount',
                        appearance: widget.appearance,
                        onClick: () => _showMoreHistories(hiddenHistoryCount),
                      ),
                    ),
                  if (!searching &&
                      _historyRenderLimit > _collapsedHistoryLimit)
                    SliverToBoxAdapter(
                      child: _HistoryLimitButton(
                        icon: Icons.keyboard_arrow_up,
                        label: '收起',
                        appearance: widget.appearance,
                        onClick: _collapseHistories,
                      ),
                    ),
                  const SliverToBoxAdapter(child: SizedBox(height: 16)),
                ],
              ),
              if (showInitialLoading)
                Positioned.fill(
                  child: IgnorePointer(
                    child: Center(
                      child: CircularProgressIndicator(
                        color: widget.appearance.statusAvailableColor,
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 16, 18),
          child: Row(
            children: <Widget>[
              Expanded(
                child: BottomSidebarAction(
                  icon: Icons.inventory_2_outlined,
                  label: '包管理',
                  appearance: widget.appearance,
                  selected: widget.selectedRouteId == packageManagerRouteId,
                  onClick: _openPackageManager,
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: BottomSidebarAction(
                  icon: Icons.settings_outlined,
                  label: '设置',
                  appearance: widget.appearance,
                  selected: widget.selectedRouteId == settingsRouteId,
                  onClick: _openSettings,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}

class _CharacterHistorySection {
  _CharacterHistorySection({
    required this.key,
    required this.label,
    required this.kind,
    required this.groups,
  });

  final String key;
  final String label;
  final _HistoryBindingKind kind;
  final List<_HistoryGroupSection> groups;

  int get historyCount {
    var count = 0;
    for (final group in groups) {
      count += group.historyCount;
    }
    return count;
  }
}

enum _HistoryBindingKind { characterCard, characterGroup, unbound }

class _HistoryGroupSection {
  _HistoryGroupSection({
    required this.key,
    required this.label,
    required this.histories,
    int? historyCount,
  }) : historyCount = historyCount ?? histories.length;

  final String key;
  final String label;
  final List<core_proxy.ChatHistory> histories;
  final int historyCount;
}

class _VisibleHistoryPlan {
  const _VisibleHistoryPlan({
    required this.sections,
    required this.hiddenCount,
  });

  final List<_CharacterHistorySection> sections;
  final int hiddenCount;
}

sealed class _HistoryListEntry {
  const _HistoryListEntry();
}

class _CharacterHeaderEntry extends _HistoryListEntry {
  const _CharacterHeaderEntry(this.section);

  final _CharacterHistorySection section;
}

class _GroupHeaderEntry extends _HistoryListEntry {
  const _GroupHeaderEntry(this.group);

  final _HistoryGroupSection group;
}

class _HistoryRowEntry extends _HistoryListEntry {
  const _HistoryRowEntry(this.history);

  final core_proxy.ChatHistory history;
}

class _ChatBindingForCreate {
  const _ChatBindingForCreate({
    required this.characterCardName,
    required this.characterGroupId,
  });

  final String? characterCardName;
  final String? characterGroupId;
}

String _shortIdentifier(String value) {
  final text = value.trim();
  if (text.length <= 12) {
    return text;
  }
  return '${text.substring(0, 8)}...${text.substring(text.length - 4)}';
}

class _CharacterSectionHeader extends StatelessWidget {
  const _CharacterSectionHeader({
    required this.label,
    required this.kind,
    required this.count,
    required this.expanded,
    required this.appearance,
    required this.onToggleExpanded,
  });

  final String label;
  final _HistoryBindingKind kind;
  final int count;
  final bool expanded;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onToggleExpanded;

  static const String _operitAvatarAsset = 'assets/images/operit_avatar.png';

  @override
  Widget build(BuildContext context) {
    final avatarContainerColor = appearance.buttonContainerColor;
    return Padding(
      padding: const EdgeInsetsDirectional.only(
        start: 20,
        end: 12,
        top: 10,
        bottom: 5,
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(18),
        onTap: onToggleExpanded,
        child: Row(
          children: <Widget>[
            Container(
              width: 22,
              height: 22,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: avatarContainerColor,
              ),
              alignment: Alignment.center,
              child:
                  kind == _HistoryBindingKind.characterCard && label == 'Operit'
                  ? ClipOval(
                      child: ColoredBox(
                        color: Colors.white,
                        child: Image.asset(
                          _operitAvatarAsset,
                          width: 20,
                          height: 20,
                          fit: BoxFit.contain,
                        ),
                      ),
                    )
                  : Icon(
                      switch (kind) {
                        _HistoryBindingKind.characterGroup =>
                          Icons.groups_outlined,
                        _HistoryBindingKind.unbound =>
                          Icons.account_tree_outlined,
                        _HistoryBindingKind.characterCard =>
                          Icons.person_outline,
                      },
                      size: 14,
                      color: appearance.itemColor,
                    ),
            ),
            const SizedBox(width: 8),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 170),
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  color: appearance.titleColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              count.toString(),
              style: Theme.of(context).textTheme.labelSmall?.copyWith(
                color: appearance.itemColor.withValues(alpha: 0.64),
                fontWeight: FontWeight.w700,
              ),
            ),
            Expanded(
              child: Container(
                height: 2,
                margin: const EdgeInsetsDirectional.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    colors: <Color>[
                      appearance.dividerColor,
                      Colors.transparent,
                    ],
                  ),
                ),
              ),
            ),
            Icon(
              expanded ? Icons.keyboard_arrow_up : Icons.keyboard_arrow_down,
              size: 23,
              color: appearance.itemColor.withValues(alpha: 0.78),
            ),
          ],
        ),
      ),
    );
  }
}

class _GroupSectionHeader extends StatelessWidget {
  const _GroupSectionHeader({
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

  static const double _endPadding = 12;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsetsDirectional.only(
        start: 22,
        end: _endPadding,
        top: 4,
        bottom: expanded ? 3 : 0,
      ),
      child: Row(
        children: <Widget>[
          HistoryRail(height: 30, appearance: appearance),
          Expanded(
            child: Material(
              color: appearance.buttonContainerColor,
              borderRadius: BorderRadius.circular(12),
              child: InkWell(
                borderRadius: BorderRadius.circular(12),
                onTap: onToggleExpanded,
                child: Padding(
                  padding: const EdgeInsetsDirectional.fromSTEB(12, 7, 10, 7),
                  child: Row(
                    children: <Widget>[
                      Icon(
                        Icons.folder_outlined,
                        size: 16,
                        color: appearance.itemColor,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          label,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.labelLarge
                              ?.copyWith(
                                color: appearance.titleColor,
                                fontWeight: FontWeight.w700,
                              ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        count.toString(),
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: appearance.itemColor.withValues(alpha: 0.72),
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(width: 4),
                      Icon(
                        expanded
                            ? Icons.keyboard_arrow_up
                            : Icons.keyboard_arrow_down,
                        size: 20,
                        color: appearance.itemColor.withValues(alpha: 0.68),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _HistoryLimitButton extends StatelessWidget {
  const _HistoryLimitButton({
    required this.icon,
    required this.label,
    required this.appearance,
    required this.onClick,
  });

  final IconData icon;
  final String label;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsetsDirectional.only(start: 24, end: 12, top: 2),
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
