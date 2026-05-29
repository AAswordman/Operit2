// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/link/CoreLinkProtocol.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../navigation/AppNavigationModels.dart';
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

  final ScrollController _historyScrollController = ScrollController();
  final GlobalKey _expandButtonKey = GlobalKey();
  final TextEditingController _searchController = TextEditingController();
  final List<core_proxy.ChatHistory> _histories = <core_proxy.ChatHistory>[];
  final Set<String> _collapsedCharacterSections = <String>{};
  final Set<String> _collapsedGroupSections = <String>{};
  StreamSubscription<List<core_proxy.ChatHistory>>? _historiesSubscription;
  StreamSubscription<String?>? _currentChatSubscription;
  String? _currentChatId;
  String? _errorMessage;
  bool _loading = true;
  bool _allHistoriesExpanded = false;
  bool _searchExpanded = false;

  GeneratedChatRuntimeHolderMainCoreProxy get _chatCoreProxy =>
      GeneratedCoreProxyClients(widget.bridge).chatRuntimeHolderMain;

  String _requestId() => 'flutter-${DateTime.now().microsecondsSinceEpoch}';

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
    _loadConversations();
    _watchConversations();
  }

  @override
  void dispose() {
    _historiesSubscription?.cancel();
    _currentChatSubscription?.cancel();
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
      final histories = await _chatCoreProxy.chatHistoriesFlowSnapshot();
      final currentChatId = await _chatCoreProxy.currentChatIdFlowSnapshot();
      if (!mounted) {
        return;
      }
      setState(() {
        _histories
          ..clear()
          ..addAll(histories);
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

  void _onSearchChanged() {
    if (mounted) {
      setState(() {});
    }
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
      await _loadConversations();
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
      await _loadConversations();
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
      await _chatCoreProxy.switchChat(chatId: history.id);
      await _loadConversations();
      widget.onConversationActivated();
    } catch (error, stackTrace) {
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
      await _loadConversations();
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
      await _loadConversations();
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
      await _loadConversations();
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
      await _loadConversations();
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
      await _loadConversations();
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
        _characterCardLabel(history).toLowerCase().contains(query) ||
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
            label: _characterCardLabel(history),
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

  String _characterSectionKey(core_proxy.ChatHistory history) {
    final name = history.characterCardName?.trim();
    return name == null || name.isEmpty
        ? 'character:unbound'
        : 'character:$name';
  }

  String _characterCardLabel(core_proxy.ChatHistory history) {
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
    });
  }

  void _toggleGroupSection(String sectionKey) {
    setState(() {
      if (_collapsedGroupSections.contains(sectionKey)) {
        _collapsedGroupSections.remove(sectionKey);
      } else {
        _collapsedGroupSections.add(sectionKey);
      }
    });
  }

  void _toggleAllHistoriesExpanded() {
    final anchorTop = _expandButtonTop;
    setState(() {
      _allHistoriesExpanded = !_allHistoriesExpanded;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          anchorTop == null ||
          !_historyScrollController.hasClients) {
        return;
      }
      final nextAnchorTop = _expandButtonTop;
      if (nextAnchorTop == null) {
        return;
      }
      final position = _historyScrollController.position;
      final targetPixels = (position.pixels + nextAnchorTop - anchorTop).clamp(
        position.minScrollExtent,
        position.maxScrollExtent,
      );
      _historyScrollController.jumpTo(targetPixels);
    });
  }

  double? get _expandButtonTop {
    final context = _expandButtonKey.currentContext;
    if (context == null) {
      return null;
    }
    final renderObject = context.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) {
      return null;
    }
    return renderObject.localToGlobal(Offset.zero).dy;
  }

  @override
  Widget build(BuildContext context) {
    final visibleHistories = _visibleHistories;
    final searching = _searchController.text.trim().isNotEmpty;
    final shownHistories = searching || _allHistoriesExpanded
        ? visibleHistories
        : visibleHistories.take(_collapsedHistoryLimit).toList(growable: false);
    final hiddenHistoryCount = visibleHistories.length - shownHistories.length;
    final characterSections = _buildCharacterSections(shownHistories);
    return Column(
      children: <Widget>[
        Expanded(
          child: Stack(
            children: <Widget>[
              SingleChildScrollView(
                controller: _historyScrollController,
                primary: false,
                padding: const EdgeInsets.fromLTRB(0, 30, 8, 16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    SidebarInfoCard(
                      brandName: 'Operit',
                      appearance: widget.appearance,
                    ),
                    const SizedBox(height: 24),
                    Padding(
                      padding: const EdgeInsetsDirectional.only(
                        start: 28,
                        end: 12,
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
                            onPressed: _toggleSearchExpanded,
                            visualDensity: VisualDensity.compact,
                            tooltip: _searchExpanded ? '收起搜索' : '搜索对话',
                            icon: Icon(
                              _searchExpanded ? Icons.search_off : Icons.search,
                              size: 20,
                              color: _searchController.text.trim().isNotEmpty
                                  ? widget.appearance.titleColor
                                  : widget.appearance.itemColor,
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(height: 6),
                    Padding(
                      padding: const EdgeInsetsDirectional.only(
                        start: 12,
                        end: 0,
                        bottom: 8,
                      ),
                      child: NewConversationButton(
                        appearance: widget.appearance,
                        onClick: _createConversation,
                        onCreateGroup: _showCreateGroupDialog,
                      ),
                    ),
                    AnimatedSize(
                      duration: const Duration(milliseconds: 180),
                      curve: Curves.easeOutCubic,
                      child: _searchExpanded
                          ? Padding(
                              padding: const EdgeInsetsDirectional.only(
                                start: 12,
                                end: 0,
                                bottom: 12,
                              ),
                              child: ConversationSearchField(
                                controller: _searchController,
                                appearance: widget.appearance,
                              ),
                            )
                          : const SizedBox.shrink(),
                    ),
                    if (_errorMessage != null)
                      SidebarStatusText(
                        text: _errorMessage!,
                        appearance: widget.appearance,
                      ),
                    for (final section in characterSections)
                      _CharacterHistorySectionView(
                        section: section,
                        selectedChatId: _currentChatId,
                        appearance: widget.appearance,
                        expanded: !_collapsedCharacterSections.contains(
                          section.key,
                        ),
                        onToggleExpanded: () =>
                            _toggleCharacterSection(section.key),
                        isGroupExpanded: (groupKey) =>
                            !_collapsedGroupSections.contains(groupKey),
                        onToggleGroupExpanded: _toggleGroupSection,
                        onHistoryClick: _switchConversation,
                        onHistoryRename: (history) {
                          _showRenameConversationDialog(history);
                        },
                        onHistoryDelete: (history) {
                          _showDeleteConversationDialog(history);
                        },
                        onHistoryLongPress: (history) {
                          _showConversationActionDialog(history);
                        },
                        onHistoryMoveTo: _moveConversationTo,
                      ),
                    if (!searching &&
                        visibleHistories.length > _collapsedHistoryLimit)
                      _ExpandSectionButton(
                        key: _expandButtonKey,
                        expanded: _allHistoriesExpanded,
                        hiddenCount: hiddenHistoryCount,
                        appearance: widget.appearance,
                        onClick: _toggleAllHistoriesExpanded,
                      ),
                  ],
                ),
              ),
              PositionedDirectional(
                top: 0,
                start: 12,
                end: 20,
                child: IgnorePointer(
                  child: AnimatedOpacity(
                    opacity: _loading ? 1 : 0,
                    duration: const Duration(milliseconds: 140),
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(999),
                      child: LinearProgressIndicator(
                        minHeight: 2,
                        color: widget.appearance.selectedContainerColor,
                        backgroundColor: widget
                            .appearance
                            .selectedContainerColor
                            .withValues(alpha: 0.12),
                      ),
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
                  onClick: () {},
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: BottomSidebarAction(
                  icon: Icons.settings_outlined,
                  label: '设置',
                  appearance: widget.appearance,
                  onClick: () {},
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
    required this.groups,
  });

  final String key;
  final String label;
  final List<_HistoryGroupSection> groups;

  int get historyCount {
    var count = 0;
    for (final group in groups) {
      count += group.histories.length;
    }
    return count;
  }
}

class _HistoryGroupSection {
  _HistoryGroupSection({
    required this.key,
    required this.label,
    required this.histories,
  });

  final String key;
  final String label;
  final List<core_proxy.ChatHistory> histories;
}

class _ChatBindingForCreate {
  const _ChatBindingForCreate({
    required this.characterCardName,
    required this.characterGroupId,
  });

  final String? characterCardName;
  final String? characterGroupId;
}

class _CharacterHistorySectionView extends StatelessWidget {
  const _CharacterHistorySectionView({
    required this.section,
    required this.selectedChatId,
    required this.appearance,
    required this.expanded,
    required this.onToggleExpanded,
    required this.isGroupExpanded,
    required this.onToggleGroupExpanded,
    required this.onHistoryClick,
    required this.onHistoryRename,
    required this.onHistoryDelete,
    required this.onHistoryLongPress,
    required this.onHistoryMoveTo,
  });

  final _CharacterHistorySection section;
  final String? selectedChatId;
  final NavigationDrawerAppearance appearance;
  final bool expanded;
  final VoidCallback onToggleExpanded;
  final bool Function(String groupKey) isGroupExpanded;
  final ValueChanged<String> onToggleGroupExpanded;
  final ValueChanged<core_proxy.ChatHistory> onHistoryClick;
  final ValueChanged<core_proxy.ChatHistory> onHistoryRename;
  final ValueChanged<core_proxy.ChatHistory> onHistoryDelete;
  final ValueChanged<core_proxy.ChatHistory> onHistoryLongPress;
  final void Function(
    core_proxy.ChatHistory moved,
    core_proxy.ChatHistory target,
  )
  onHistoryMoveTo;

  @override
  Widget build(BuildContext context) {
    final children = <Widget>[
      _CharacterSectionHeader(
        label: section.label,
        count: section.historyCount,
        expanded: expanded,
        appearance: appearance,
        onToggleExpanded: onToggleExpanded,
      ),
    ];

    if (expanded) {
      for (final group in section.groups) {
        final groupExpanded = isGroupExpanded(group.key);
        children.add(
          _GroupSectionHeader(
            label: group.label,
            count: group.histories.length,
            expanded: groupExpanded,
            appearance: appearance,
            onToggleExpanded: () => onToggleGroupExpanded(group.key),
          ),
        );
        if (!groupExpanded) {
          continue;
        }
        for (final history in group.histories) {
          children.add(
            ConversationDrawerItem(
              history: history,
              title: history.title,
              selected: selectedChatId == history.id,
              appearance: appearance,
              nested: true,
              onClick: () => onHistoryClick(history),
              onRename: () => onHistoryRename(history),
              onDelete: () => onHistoryDelete(history),
              onLongPress: () => onHistoryLongPress(history),
              onMoveTo: (moved) => onHistoryMoveTo(moved, history),
            ),
          );
        }
      }
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: children,
    );
  }
}

class _CharacterSectionHeader extends StatelessWidget {
  const _CharacterSectionHeader({
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

  static const String _operitAvatarAsset = 'assets/images/operit_avatar.png';

  @override
  Widget build(BuildContext context) {
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
            DecoratedBox(
              decoration: BoxDecoration(
                color: appearance.selectedContainerColor.withValues(
                  alpha: 0.24,
                ),
                borderRadius: const BorderRadiusDirectional.only(
                  topStart: Radius.circular(5),
                  bottomStart: Radius.circular(5),
                  topEnd: Radius.circular(18),
                  bottomEnd: Radius.circular(18),
                ),
              ),
              child: Padding(
                padding: const EdgeInsetsDirectional.fromSTEB(7, 4, 12, 4),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Container(
                      width: 22,
                      height: 22,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: appearance.selectedContainerColor.withValues(
                          alpha: 0.38,
                        ),
                      ),
                      alignment: Alignment.center,
                      child: label == 'Operit'
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
                              label == '未绑定'
                                  ? Icons.account_tree_outlined
                                  : Icons.person_outline,
                              size: 14,
                              color: appearance.titleColor,
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
                        color: appearance.titleColor.withValues(alpha: 0.58),
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Expanded(
              child: Container(
                height: 2,
                margin: const EdgeInsetsDirectional.symmetric(horizontal: 10),
                decoration: BoxDecoration(
                  gradient: LinearGradient(
                    colors: <Color>[
                      appearance.selectedContainerColor.withValues(alpha: 0.52),
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

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsetsDirectional.only(
        start: 22,
        end: 0,
        top: 4,
        bottom: expanded ? 3 : 0,
      ),
      child: Row(
        children: <Widget>[
          HistoryRail(height: 30, appearance: appearance),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: appearance.selectedContainerColor.withValues(
                  alpha: 0.13,
                ),
                borderRadius: BorderRadius.circular(13),
                border: Border.all(
                  color: appearance.selectedContainerColor.withValues(
                    alpha: 0.12,
                  ),
                ),
              ),
              child: Material(
                color: Colors.transparent,
                borderRadius: BorderRadius.circular(13),
                child: InkWell(
                  borderRadius: BorderRadius.circular(13),
                  onTap: onToggleExpanded,
                  child: Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(10, 6, 9, 6),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.folder_outlined,
                          size: 16,
                          color: appearance.titleColor.withValues(alpha: 0.78),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            label,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.labelLarge
                                ?.copyWith(
                                  color: appearance.titleColor.withValues(
                                    alpha: 0.86,
                                  ),
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          count.toString(),
                          style: Theme.of(context).textTheme.labelSmall
                              ?.copyWith(
                                color: appearance.itemColor.withValues(
                                  alpha: 0.54,
                                ),
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
          ),
        ],
      ),
    );
  }
}

class _ExpandSectionButton extends StatelessWidget {
  const _ExpandSectionButton({
    super.key,
    required this.expanded,
    required this.hiddenCount,
    required this.appearance,
    required this.onClick,
  });

  final bool expanded;
  final int hiddenCount;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsetsDirectional.only(start: 24, end: 0, top: 2),
      child: TextButton.icon(
        onPressed: onClick,
        icon: Icon(
          expanded ? Icons.expand_less : Icons.expand_more,
          size: 18,
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
        label: Text(
          expanded ? '收起' : '展开更多 $hiddenCount',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
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
