// ignore_for_file: file_names

part of 'DrawerContent.dart';

class _NativeWorkspaceGroup {
  const _NativeWorkspaceGroup({
    required this.key,
    required this.workspaceId,
    required this.name,
    required this.histories,
  });

  final String key;
  final String? workspaceId;
  final String? name;
  final List<core_proxy.ChatHistoryListItem> histories;

  String get label => name ?? '未分组';
  SidebarDirectory get directory =>
      SidebarDirectory.group(workspaceId: workspaceId, groupName: name);
}

enum _GroupQuickAction {
  createChat,
  createGroup,
  rename,
  togglePinned,
  dissolve,
}

extension _NativeWorkspaceGroups on _DrawerContentState {
  List<_NativeWorkspaceGroup> _nativeGroups(_WorkspaceSection section) {
    final groups = <String?, List<core_proxy.ChatHistoryListItem>>{};
    for (final chat in section.histories) {
      groups.putIfAbsent(_normalizedGroupName(chat), () => []).add(chat);
    }
    final result = [
      for (final entry in groups.entries)
        _NativeWorkspaceGroup(
          key: entry.key == null
              ? section.key
              : '${section.key}::group:${entry.key}',
          workspaceId: section.workspaceId,
          name: entry.key,
          histories: entry.value,
        ),
    ];
    return [
      ...result.where((group) => _pinnedDirectories.contains(group.directory)),
      ...result.where((group) => !_pinnedDirectories.contains(group.directory)),
    ];
  }

  String? _normalizedGroupName(core_proxy.ChatHistoryListItem chat) {
    final name = chat.group?.trim();
    return name == null || name.isEmpty ? null : name;
  }

  List<core_proxy.ChatHistoryListItem> _groupMembers(
    _WorkspaceSection section,
    String? name,
  ) => widget.histories
      .where(
        (chat) =>
            _workspaceKey(chat) == section.key &&
            _normalizedGroupName(chat) == name,
      )
      .toList(growable: false);

  List<core_proxy.ChatHistoryListItem> _groupPreview(
    List<core_proxy.ChatHistoryListItem> chats,
  ) => [
    for (var i = 0; i < chats.length; i++)
      if (i < _DrawerContentState._previewLimit ||
          chats[i].pinned ||
          chats[i].id == widget.currentChatId ||
          widget.activeStreamingChatIds.contains(chats[i].id))
        chats[i],
  ];

  Future<void> _runGroupMutation(Future<void> Function() mutation) async {
    if (_groupMutationPending) return;
    _groupMutationPending = true;
    try {
      await mutation();
    } catch (error) {
      if (mounted) {
        _showGroupError(error);
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(error.toString())));
      }
    } finally {
      _groupMutationPending = false;
    }
  }

  Future<String?> _groupNameDialog(
    String title,
    String name, {
    String labelText = '分组名称',
  }) async {
    var value = name;
    return showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: TextFormField(
          initialValue: name,
          autofocus: true,
          decoration: InputDecoration(labelText: labelText),
          onChanged: (text) => value = text,
          onFieldSubmitted: (text) {
            if (text.trim().isNotEmpty) Navigator.pop(context, text.trim());
          },
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              if (value.trim().isNotEmpty) Navigator.pop(context, value.trim());
            },
            child: const Text('确认'),
          ),
        ],
      ),
    );
  }

  void _checkGroupNameAvailable(
    _WorkspaceSection section,
    String name, {
    String? replacing,
  }) {
    final duplicate = widget.histories.any(
      (chat) =>
          _workspaceKey(chat) == section.key &&
          _normalizedGroupName(chat) == name &&
          name != replacing,
    );
    if (duplicate) {
      throw StateError('当前项目已存在分组“$name”');
    }
  }

  Future<void> _createWorkspaceGroup(_WorkspaceSection section) async {
    if (!section.canCreate) {
      _showGroupError(StateError('项目记录缺失，无法新建分组'));
      return;
    }
    final name = await _groupNameDialog('新建分组', '');
    if (name == null || !mounted) return;
    try {
      _checkGroupNameAvailable(section, name);
    } catch (error) {
      _showGroupError(error);
      return;
    }
    await _runGroupMutation(() => _createGroupedConversation(section, name));
  }

  Future<void> _createGroupedConversation(
    _WorkspaceSection section,
    String? name,
  ) async {
    if (!section.canCreate) {
      throw StateError('项目记录缺失，无法新建对话');
    }
    final id = await _chatCoreProxy.createNewChat(
      setAsCurrentChat: false,
      sourceChatId: null,
      parentChatId: null,
      workspaceId: section.workspaceId,
      input: null,
    );
    await _chatCoreProxy.updateChatGroups(chatIds: [id], groupName: name);
    newChatIntroArmed.value = true;
    await _activateChat(id);
  }

  Future<void> _renameWorkspaceGroup(
    _WorkspaceSection section,
    _NativeWorkspaceGroup group,
  ) async {
    final name = group.name;
    if (name == null) return;
    final renamed = await _groupNameDialog('重命名分组', name);
    if (renamed == null || renamed == name || !mounted) return;
    try {
      _checkGroupNameAvailable(section, renamed, replacing: name);
    } catch (error) {
      _showGroupError(error);
      return;
    }
    final members = _groupMembers(section, name);
    await _runGroupMutation(() async {
      await _chatCoreProxy.updateChatGroups(
        chatIds: members.map((chat) => chat.id).toList(growable: false),
        groupName: renamed,
      );
      await _directoryPreferences.moveDirectoryPin(
        from: group.directory,
        to: SidebarDirectory.group(
          workspaceId: section.workspaceId,
          groupName: renamed,
        ),
      );
    });
  }

  Future<void> _dissolveWorkspaceGroup(
    _WorkspaceSection section,
    _NativeWorkspaceGroup group,
  ) async {
    final members = _groupMembers(section, group.name);
    await _runGroupMutation(() async {
      await _chatCoreProxy.updateChatGroups(
        chatIds: members.map((chat) => chat.id).toList(growable: false),
        groupName: null,
      );
      await _directoryPreferences.setDirectoryPinned(group.directory, false);
    });
  }

  Future<void> _moveWorkspaceGroup(
    core_proxy.ChatHistoryListItem chat,
    String? name,
  ) => _chatCoreProxy.updateChatGroups(chatIds: [chat.id], groupName: name);

  Future<void> _toggleDirectoryPinned(SidebarDirectory directory) {
    return _runGroupMutation(
      () => _directoryPreferences.setDirectoryPinned(
        directory,
        !_pinnedDirectories.contains(directory),
      ),
    );
  }
}

class _GroupSectionHeader extends StatefulWidget {
  const _GroupSectionHeader({
    super.key,
    required this.group,
    required this.pinned,
    required this.expanded,
    required this.appearance,
    required this.canCreate,
    required this.onToggleExpanded,
    required this.onCreateChat,
    required this.onCreateGroup,
    required this.onRename,
    required this.onTogglePinned,
    required this.onDissolve,
    required this.canAcceptDrop,
    required this.onMoveToGroup,
  });

  final _NativeWorkspaceGroup group;
  final bool pinned;
  final bool expanded;
  final NavigationDrawerAppearance appearance;
  final bool canCreate;
  final VoidCallback onToggleExpanded;
  final VoidCallback onCreateChat;
  final VoidCallback onCreateGroup;
  final VoidCallback onRename;
  final VoidCallback onTogglePinned;
  final VoidCallback onDissolve;
  final bool Function(core_proxy.ChatHistoryListItem) canAcceptDrop;
  final ValueChanged<core_proxy.ChatHistoryListItem> onMoveToGroup;

  @override
  State<_GroupSectionHeader> createState() => _GroupSectionHeaderState();
}

class _GroupSectionHeaderState extends State<_GroupSectionHeader> {
  bool _hovered = false;
  bool _menuOpen = false;

  void _select(_GroupQuickAction action) {
    switch (action) {
      case _GroupQuickAction.createChat:
        widget.onCreateChat();
      case _GroupQuickAction.createGroup:
        widget.onCreateGroup();
      case _GroupQuickAction.rename:
        widget.onRename();
      case _GroupQuickAction.togglePinned:
        widget.onTogglePinned();
      case _GroupQuickAction.dissolve:
        widget.onDissolve();
    }
  }

  @override
  Widget build(BuildContext context) {
    final appearance = widget.appearance;
    final platform = Theme.of(context).platform;
    final touchPlatform =
        platform == TargetPlatform.android || platform == TargetPlatform.iOS;
    final showActions = _hovered || _menuOpen || touchPlatform;
    return DragTarget<core_proxy.ChatHistoryListItem>(
      onWillAcceptWithDetails: (details) => widget.canAcceptDrop(details.data),
      onAcceptWithDetails: (details) => widget.onMoveToGroup(details.data),
      builder: (context, candidateData, rejectedData) {
        return MouseRegion(
          onEnter: (_) => setState(() => _hovered = true),
          onExit: (_) => setState(() => _hovered = false),
          child: Padding(
            padding: EdgeInsetsDirectional.only(
              start: 44,
              end: 12,
              top: 2,
              bottom: widget.expanded ? 2 : 0,
            ),
            child: DecoratedBox(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(8),
                border: candidateData.isEmpty
                    ? null
                    : Border.all(
                        color: appearance.statusAvailableColor.withValues(
                          alpha: 0.55,
                        ),
                      ),
              ),
              child: Material(
                color: Colors.transparent,
                borderRadius: BorderRadius.circular(8),
                child: InkWell(
                  borderRadius: BorderRadius.circular(8),
                  onTap: widget.onToggleExpanded,
                  child: Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(8, 4, 6, 4),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.folder_outlined,
                          size: 14,
                          color: appearance.itemColor.withValues(alpha: 0.76),
                        ),
                        const SizedBox(width: 7),
                        Expanded(
                          child: Text(
                            widget.group.label,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.bodySmall
                                ?.copyWith(
                                  color: appearance.titleColor.withValues(
                                    alpha: 0.86,
                                  ),
                                  fontWeight: FontWeight.w600,
                                ),
                          ),
                        ),
                        if (widget.pinned) ...<Widget>[
                          const SizedBox(width: 4),
                          Icon(
                            Icons.push_pin_rounded,
                            size: 12,
                            color: appearance.itemColor.withValues(alpha: 0.65),
                          ),
                        ],
                        const SizedBox(width: 2),
                        AnimatedOpacity(
                          duration: const Duration(milliseconds: 140),
                          opacity: showActions ? 1 : 0,
                          child: IgnorePointer(
                            ignoring: !showActions,
                            child: SidebarMoreMenuButton<_GroupQuickAction>(
                              compact: true,
                              tooltip: '分组操作',
                              color: appearance.itemColor.withValues(
                                alpha: 0.78,
                              ),
                              onMenuOpenChanged: (open) {
                                if (mounted) setState(() => _menuOpen = open);
                              },
                              onSelected: _select,
                              actions: <SidebarMenuAction<_GroupQuickAction>>[
                                SidebarMenuAction(
                                  value: _GroupQuickAction.createChat,
                                  icon: Icons.add_comment_outlined,
                                  label: '新建对话',
                                  enabled: widget.canCreate,
                                ),
                                SidebarMenuAction(
                                  value: _GroupQuickAction.createGroup,
                                  icon: Icons.create_new_folder_outlined,
                                  label: '新建分组',
                                  enabled: widget.canCreate,
                                ),
                                if (widget.group.name != null)
                                  const SidebarMenuAction(
                                    value: _GroupQuickAction.rename,
                                    icon: Icons.edit_outlined,
                                    label: '重命名',
                                  ),
                                SidebarMenuAction(
                                  value: _GroupQuickAction.togglePinned,
                                  icon: widget.pinned
                                      ? Icons.push_pin_outlined
                                      : Icons.push_pin_rounded,
                                  label: widget.pinned ? '取消置顶' : '置顶',
                                ),
                                if (widget.group.name != null)
                                  const SidebarMenuAction(
                                    value: _GroupQuickAction.dissolve,
                                    icon: Icons.folder_off_outlined,
                                    label: '解散分组',
                                    dividerBefore: true,
                                  ),
                              ],
                            ),
                          ),
                        ),
                        const SizedBox(width: 2),
                        Icon(
                          widget.expanded
                              ? Icons.expand_less
                              : Icons.expand_more,
                          size: 17,
                          color: appearance.itemColor.withValues(alpha: 0.62),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
