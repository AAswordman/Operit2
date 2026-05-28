// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/bridge/OperitRuntimeBridge.dart';
import '../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../core/link/CoreLinkProtocol.dart';
import '../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../navigation/AppNavigationModels.dart';
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
    final controller = TextEditingController();
    final groupName = await showDialog<String>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text('新建分组'),
          content: TextField(
            controller: controller,
            autofocus: true,
            decoration: const InputDecoration(labelText: '分组名称'),
            onSubmitted: (value) => Navigator.of(context).pop(value),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(controller.text),
              child: const Text('创建'),
            ),
          ],
        );
      },
    );
    controller.dispose();
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
      final groupKey = _groupSectionKey(history);
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

  String _groupSectionKey(core_proxy.ChatHistory history) {
    final group = history.group?.trim();
    return group == null || group.isEmpty ? 'group:ungrouped' : 'group:$group';
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
                    _SidebarInfoCard(
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
                      child: _NewConversationButton(
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
                              child: _ConversationSearchField(
                                controller: _searchController,
                                appearance: widget.appearance,
                              ),
                            )
                          : const SizedBox.shrink(),
                    ),
                    if (_errorMessage != null)
                      _SidebarStatusText(
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
                child: _BottomSidebarAction(
                  icon: Icons.inventory_2_outlined,
                  label: '包管理',
                  appearance: widget.appearance,
                  onClick: () {},
                ),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: _BottomSidebarAction(
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

class CollapsedDrawerContent extends StatelessWidget {
  const CollapsedDrawerContent({
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
  static const double _topBarHeight = 64;
  static const String _operitLogoAsset =
      'assets/images/operit_logo_transparent.png';

  Future<void> _createConversation() async {
    await GeneratedCoreProxyClients(bridge).chatRuntimeHolderMain.createNewChat(
      characterCardName: null,
      group: null,
      inheritGroupFromCurrent: true,
      setAsCurrentChat: true,
      characterGroupId: null,
    );
    onConversationActivated();
  }

  @override
  Widget build(BuildContext context) {
    final topPadding = MediaQuery.paddingOf(context).top;
    return ListView(
      padding: const EdgeInsets.only(bottom: 24),
      children: <Widget>[
        SizedBox(
          height: topPadding + _topBarHeight,
          child: Padding(
            padding: EdgeInsets.only(top: topPadding),
            child: Center(
              child: Image.asset(
                _operitLogoAsset,
                width: 34,
                height: 34,
                fit: BoxFit.contain,
              ),
            ),
          ),
        ),
        const SizedBox(height: 24),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Center(
            child: _RoundDrawerButton(
              selected: selectedRouteId == navigationEntries.first.routeId,
              appearance: appearance,
              icon: Icons.chat_bubble_outline,
              onClick: onConversationActivated,
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 8),
          child: Center(
            child: _RoundDrawerButton(
              selected: false,
              appearance: appearance,
              icon: Icons.add_comment_outlined,
              onClick: _createConversation,
            ),
          ),
        ),
        const SizedBox(height: 16),
        Center(
          child: _RoundDrawerButton(
            selected: false,
            appearance: appearance,
            icon: Icons.inventory_2_outlined,
            onClick: () {},
          ),
        ),
        const SizedBox(height: 8),
        Center(
          child: _RoundDrawerButton(
            selected: false,
            appearance: appearance,
            icon: Icons.settings_outlined,
            onClick: () {},
          ),
        ),
      ],
    );
  }
}

class _SidebarInfoCard extends StatelessWidget {
  const _SidebarInfoCard({required this.brandName, required this.appearance});

  final String brandName;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            brandName,
            style: Theme.of(context).textTheme.titleLarge?.copyWith(
              letterSpacing: 0,
              color: appearance.titleColor,
              fontWeight: FontWeight.bold,
            ),
          ),
        ],
      ),
    );
  }
}

class _NewConversationButton extends StatelessWidget {
  const _NewConversationButton({
    required this.appearance,
    required this.onClick,
    required this.onCreateGroup,
  });

  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final VoidCallback onCreateGroup;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(16);
    return Row(
      children: <Widget>[
        Expanded(
          child: Material(
            color: appearance.selectedContainerColor.withValues(alpha: 0.30),
            borderRadius: shape,
            child: InkWell(
              borderRadius: shape,
              onTap: onClick,
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 12,
                ),
                child: Row(
                  children: <Widget>[
                    Icon(Icons.add, size: 21, color: appearance.itemColor),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        '新建对话',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: appearance.itemColor,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        SizedBox(
          width: 44,
          height: 44,
          child: Material(
            color: Colors.transparent,
            borderRadius: BorderRadius.circular(22),
            child: IconButton(
              onPressed: onCreateGroup,
              icon: Icon(
                Icons.add_circle_outline,
                size: 24,
                color: appearance.titleColor,
              ),
              tooltip: '新建分组',
              style: IconButton.styleFrom(
                shape: const CircleBorder(),
                backgroundColor: Colors.transparent,
                foregroundColor: appearance.titleColor,
                overlayColor: appearance.selectedContainerColor.withValues(
                  alpha: 0.20,
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _ConversationSearchField extends StatelessWidget {
  const _ConversationSearchField({
    required this.controller,
    required this.appearance,
  });

  final TextEditingController controller;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(14);
    return TextField(
      controller: controller,
      minLines: 1,
      maxLines: 1,
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: appearance.titleColor),
      decoration: InputDecoration(
        isDense: true,
        hintText: '搜索对话',
        hintStyle: Theme.of(context).textTheme.bodyMedium?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.62),
        ),
        prefixIcon: Icon(
          Icons.search,
          size: 20,
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
        filled: true,
        fillColor: appearance.selectedContainerColor.withValues(alpha: 0.16),
        border: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide.none,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: shape,
          borderSide: BorderSide(color: appearance.selectedContainerColor),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 12,
          vertical: 10,
        ),
      ),
    );
  }
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
  });

  final _CharacterHistorySection section;
  final String? selectedChatId;
  final NavigationDrawerAppearance appearance;
  final bool expanded;
  final VoidCallback onToggleExpanded;
  final bool Function(String groupKey) isGroupExpanded;
  final ValueChanged<String> onToggleGroupExpanded;
  final ValueChanged<core_proxy.ChatHistory> onHistoryClick;

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
            _ConversationDrawerItem(
              title: history.title,
              selected: selectedChatId == history.id,
              appearance: appearance,
              nested: true,
              onClick: () => onHistoryClick(history),
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
        top: 16,
        bottom: 8,
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
                padding: const EdgeInsetsDirectional.fromSTEB(8, 6, 14, 6),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Container(
                      width: 26,
                      height: 26,
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
                                  width: 24,
                                  height: 24,
                                  fit: BoxFit.contain,
                                ),
                              ),
                            )
                          : Icon(
                              label == '未绑定'
                                  ? Icons.account_tree_outlined
                                  : Icons.person_outline,
                              size: 16,
                              color: appearance.titleColor,
                            ),
                    ),
                    const SizedBox(width: 9),
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
        bottom: expanded ? 6 : 0,
      ),
      child: Row(
        children: <Widget>[
          _HistoryRail(height: 38, appearance: appearance),
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
                    padding: const EdgeInsetsDirectional.fromSTEB(11, 8, 10, 8),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.folder_outlined,
                          size: 18,
                          color: appearance.titleColor.withValues(alpha: 0.78),
                        ),
                        const SizedBox(width: 9),
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

class _HistoryRail extends StatelessWidget {
  const _HistoryRail({required this.height, required this.appearance});

  final double height;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 24,
      height: height,
      child: Center(
        child: Container(
          width: 2,
          height: height,
          decoration: BoxDecoration(
            color: appearance.selectedContainerColor.withValues(alpha: 0.36),
            borderRadius: BorderRadius.circular(1),
          ),
        ),
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

class _ConversationDrawerItem extends StatelessWidget {
  const _ConversationDrawerItem({
    required this.title,
    required this.selected,
    required this.appearance,
    required this.onClick,
    this.nested = false,
  });

  final String title;
  final bool selected;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onClick;
  final bool nested;

  @override
  Widget build(BuildContext context) {
    final itemShape = BorderRadius.circular(16);
    return Padding(
      padding: EdgeInsetsDirectional.only(
        start: nested ? 22 : 12,
        end: 0,
        bottom: 6,
      ),
      child: Row(
        children: <Widget>[
          if (nested) _HistoryRail(height: 50, appearance: appearance),
          Expanded(
            child: Material(
              color: selected
                  ? appearance.selectedContainerColor
                  : Colors.transparent,
              borderRadius: itemShape,
              child: InkWell(
                borderRadius: itemShape,
                onTap: onClick,
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 12,
                  ),
                  child: Row(
                    children: <Widget>[
                      SizedBox(
                        width: 40,
                        height: 40,
                        child: IconButton(
                          onPressed: () {},
                          icon: Icon(
                            Icons.drag_handle,
                            size: 24,
                            color: selected
                                ? appearance.selectedContentColor
                                : appearance.itemColor,
                          ),
                          tooltip: '拖动对话',
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: Theme.of(context).textTheme.bodyMedium
                              ?.copyWith(
                                color: selected
                                    ? appearance.selectedContentColor
                                    : appearance.itemColor,
                                fontWeight: selected
                                    ? FontWeight.w600
                                    : FontWeight.w400,
                              ),
                        ),
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

class _SidebarStatusText extends StatelessWidget {
  const _SidebarStatusText({required this.text, required this.appearance});

  final String text;
  final NavigationDrawerAppearance appearance;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsetsDirectional.fromSTEB(28, 6, 16, 10),
      child: Text(
        text,
        maxLines: 3,
        overflow: TextOverflow.ellipsis,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: appearance.itemColor.withValues(alpha: 0.72),
        ),
      ),
    );
  }
}

class _BottomSidebarAction extends StatelessWidget {
  const _BottomSidebarAction({
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
    final shape = BorderRadius.circular(14);
    return Material(
      color: appearance.selectedContainerColor.withValues(alpha: 0.18),
      borderRadius: shape,
      child: InkWell(
        borderRadius: shape,
        onTap: onClick,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: <Widget>[
              Icon(icon, size: 18, color: appearance.itemColor),
              const SizedBox(width: 6),
              Flexible(
                child: Text(
                  label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                    color: appearance.itemColor,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _RoundDrawerButton extends StatelessWidget {
  const _RoundDrawerButton({
    required this.selected,
    required this.appearance,
    required this.icon,
    required this.onClick,
  });

  final bool selected;
  final NavigationDrawerAppearance appearance;
  final IconData icon;
  final VoidCallback onClick;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 40,
      height: 40,
      child: Material(
        color: selected ? appearance.selectedContainerColor : Colors.transparent,
        shape: const CircleBorder(),
        child: IconButton(
          onPressed: onClick,
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(width: 40, height: 40),
          iconSize: 20,
          icon: Icon(
            icon,
            color: selected
                ? appearance.selectedContentColor
                : appearance.itemColor,
          ),
        ),
      ),
    );
  }
}
