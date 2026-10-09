// ignore_for_file: file_names

part of 'DrawerContent.dart';

/// Pure native grouping over canonical chat metadata; no plugin catalog or UI route.
class _NativeWorkspaceGroup {
  _NativeWorkspaceGroup({
    required this.key,
    required this.name,
    required this.histories,
  });
  final String key;
  final String? name;
  final List<core_proxy.ChatHistoryListItem> histories;
  String get label => name ?? '未分组';
}

extension _NativeWorkspaceGroups on _DrawerContentState {
  List<_NativeWorkspaceGroup> _nativeGroups(_WorkspaceSection section) {
    final groups = <String?, List<core_proxy.ChatHistoryListItem>>{};
    for (final chat in section.histories) {
      final name = chat.group?.trim();
      groups
          .putIfAbsent(name == null || name.isEmpty ? null : name, () => [])
          .add(chat);
    }
    return [
      for (final entry in groups.entries)
        _NativeWorkspaceGroup(
          key: entry.key == null
              ? section.key
              : '${section.key}::group:${entry.key}',
          name: entry.key,
          histories: entry.value,
        ),
    ];
  }

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

  Future<String?> _groupNameDialog(String title, String name) async {
    var value = name;
    return showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(title),
        content: TextFormField(
          initialValue: name,
          autofocus: true,
          decoration: const InputDecoration(labelText: '分组名称'),
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

  Future<void> _createWorkspaceGroup() async {
    final name = await _groupNameDialog('新建分组', '');
    if (name == null || !mounted) return;
    final source = widget.histories
        .where((chat) => chat.id == widget.currentChatId)
        .firstOrNull;
    final key = source == null ? 'workspace:unbound' : _workspaceKey(source);
    final section = _WorkspaceSection(key: key, label: '');
    section.histories.addAll(
      widget.histories.where((chat) => _workspaceKey(chat) == key),
    );
    await _runGroupMutation(() => _createGroupedConversation(section, name));
  }

  Future<void> _createGroupedConversation(
    _WorkspaceSection section,
    String? name,
  ) async {
    final current = section.histories
        .where((chat) => chat.id == widget.currentChatId)
        .firstOrNull;
    final source = current ?? section.histories.firstOrNull;
    final id = await _chatCoreProxy.createNewChat(
      setAsCurrentChat: false,
      sourceChatId: source?.id,
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
    final name = await _groupNameDialog('重命名分组', group.label);
    if (name == null || !mounted) return;
    // Use full membership, not just search-filtered/preview rows.
    final ids = widget.histories
        .where(
          (chat) =>
              _workspaceKey(chat) == section.key &&
              (chat.group?.trim().isEmpty ?? true
                      ? null
                      : chat.group?.trim()) ==
                  group.name,
        )
        .map((chat) => chat.id)
        .toList();
    await _runGroupMutation(
      () => _chatCoreProxy.updateChatGroups(chatIds: ids, groupName: name),
    );
  }

  Future<void> _deleteWorkspaceGroup(_NativeWorkspaceGroup group) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('删除分组'),
        content: Text('确认删除“${group.label}”及其中的全部对话？此操作不可撤销。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) return;
    final first = group.histories.firstOrNull;
    if (first == null) return;
    final members = widget.histories
        .where(
          (chat) =>
              _workspaceKey(chat) == _workspaceKey(first) &&
              (chat.group?.trim().isEmpty ?? true
                      ? null
                      : chat.group?.trim()) ==
                  group.name,
        )
        .toList();
    await _runGroupMutation(() async {
      for (final chat in members) {
        if (!await _chatCoreProxy.deleteChatHistory(chatId: chat.id))
          throw StateError('删除对话失败：${chat.id}');
      }
    });
  }

  Future<void> _moveWorkspaceGroup(
    core_proxy.ChatHistoryListItem chat,
    String? name,
  ) => _chatCoreProxy.updateChatGroups(chatIds: [chat.id], groupName: name);
}

enum _GroupQuickAction { createChat, rename, togglePinned, delete }

class _GroupSectionHeader extends StatefulWidget {
  /// Creates a collapsible group header for history entries.
  const _GroupSectionHeader({
    super.key,
    required this.label,
    required this.pinned,
    required this.workspaceStyle,
    required this.expanded,
    required this.appearance,
    required this.onToggleExpanded,
    required this.onCreateChat,
    required this.onRename,
    required this.onTogglePinned,
    required this.onDelete,
    required this.canAcceptDrop,
    required this.onMoveToGroup,
  });

  final String label;
  final bool pinned;
  final bool workspaceStyle;
  final bool expanded;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onToggleExpanded;
  final VoidCallback onCreateChat;
  final VoidCallback onRename;
  final VoidCallback onTogglePinned;
  final VoidCallback onDelete;
  final bool Function(core_proxy.ChatHistoryListItem) canAcceptDrop;
  final ValueChanged<core_proxy.ChatHistoryListItem> onMoveToGroup;

  @override
  State<_GroupSectionHeader> createState() => _GroupSectionHeaderState();
}

class _GroupSectionHeaderState extends State<_GroupSectionHeader> {
  static const double _endPadding = 12;

  bool _hovered = false;
  bool _menuOpen = false;

  /// Builds a history group header in the current drawer style.
  @override
  Widget build(BuildContext context) {
    final appearance = widget.appearance;
    final workspaceStyle = widget.workspaceStyle;
    final expanded = widget.expanded;
    final platform = Theme.of(context).platform;
    final touchPlatform =
        platform == TargetPlatform.android || platform == TargetPlatform.iOS;
    final showActions = _hovered || _menuOpen || touchPlatform;

    return DragTarget<core_proxy.ChatHistoryListItem>(
      onWillAcceptWithDetails: (details) => widget.canAcceptDrop(details.data),
      onAcceptWithDetails: (details) => widget.onMoveToGroup(details.data),
      builder: (context, candidateData, rejectedData) {
        final dragHovering = candidateData.isNotEmpty;
        final border = dragHovering
            ? Border.all(
                color: appearance.statusAvailableColor.withValues(alpha: 0.55),
              )
            : null;

        if (workspaceStyle) {
          return MouseRegion(
            onEnter: (_) => setState(() => _hovered = true),
            onExit: (_) => setState(() => _hovered = false),
            child: Padding(
              padding: EdgeInsetsDirectional.only(
                start: 44,
                end: _endPadding,
                top: 2,
                bottom: expanded ? 2 : 0,
              ),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(8),
                  border: border,
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
                              widget.label,
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
                              color: appearance.itemColor.withValues(
                                alpha: 0.65,
                              ),
                            ),
                          ],
                          const SizedBox(width: 2),
                          AnimatedOpacity(
                            duration: const Duration(milliseconds: 140),
                            opacity: showActions ? 1.0 : 0.0,
                            child: IgnorePointer(
                              ignoring: !showActions,
                              child: _GroupMoreMenuButton(
                                pinned: widget.pinned,
                                appearance: appearance,
                                compact: true,
                                onCreateChat: widget.onCreateChat,
                                onRename: widget.onRename,
                                onTogglePinned: widget.onTogglePinned,
                                onDelete: widget.onDelete,
                                onMenuOpenChanged: (open) {
                                  if (mounted) {
                                    setState(() => _menuOpen = open);
                                  }
                                },
                              ),
                            ),
                          ),
                          const SizedBox(width: 2),
                          Icon(
                            expanded ? Icons.expand_less : Icons.expand_more,
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
        }

        return MouseRegion(
          onEnter: (_) => setState(() => _hovered = true),
          onExit: (_) => setState(() => _hovered = false),
          child: Padding(
            padding: EdgeInsetsDirectional.only(
              start: 46,
              end: _endPadding,
              top: 4,
              bottom: expanded ? 2 : 0,
            ),
            child: DecoratedBox(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(12),
                border: border,
              ),
              child: Material(
                color: appearance.buttonContainerColor,
                borderRadius: BorderRadius.circular(12),
                child: InkWell(
                  borderRadius: BorderRadius.circular(12),
                  onTap: widget.onToggleExpanded,
                  child: Padding(
                    padding: const EdgeInsetsDirectional.fromSTEB(12, 6, 8, 6),
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
                            widget.label,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: Theme.of(context).textTheme.labelLarge
                                ?.copyWith(
                                  color: appearance.titleColor,
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                        ),
                        if (widget.pinned) ...<Widget>[
                          const SizedBox(width: 4),
                          Icon(
                            Icons.push_pin_rounded,
                            size: 12,
                            color: appearance.itemColor.withValues(alpha: 0.68),
                          ),
                        ],
                        const SizedBox(width: 2),
                        AnimatedOpacity(
                          duration: const Duration(milliseconds: 140),
                          opacity: showActions ? 1.0 : 0.0,
                          child: IgnorePointer(
                            ignoring: !showActions,
                            child: _GroupMoreMenuButton(
                              pinned: widget.pinned,
                              appearance: appearance,
                              onCreateChat: widget.onCreateChat,
                              onRename: widget.onRename,
                              onTogglePinned: widget.onTogglePinned,
                              onDelete: widget.onDelete,
                              onMenuOpenChanged: (open) {
                                if (mounted) {
                                  setState(() => _menuOpen = open);
                                }
                              },
                            ),
                          ),
                        ),
                        const SizedBox(width: 2),
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
        );
      },
    );
  }
}

class _GroupMoreMenuButton extends StatelessWidget {
  const _GroupMoreMenuButton({
    required this.pinned,
    required this.appearance,
    required this.onCreateChat,
    required this.onRename,
    required this.onTogglePinned,
    required this.onDelete,
    this.onMenuOpenChanged,
    this.compact = false,
  });

  final bool pinned;
  final NavigationDrawerAppearance appearance;
  final VoidCallback onCreateChat;
  final VoidCallback onRename;
  final VoidCallback onTogglePinned;
  final VoidCallback onDelete;
  final ValueChanged<bool>? onMenuOpenChanged;
  final bool compact;

  PopupMenuItem<_GroupQuickAction> _menuItem({
    required _GroupQuickAction value,
    required IconData icon,
    required String label,
    required Color iconColor,
    required Color textColor,
  }) {
    return PopupMenuItem<_GroupQuickAction>(
      value: value,
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 14.5, color: iconColor),
          const SizedBox(width: 9),
          Text(
            label,
            style: TextStyle(
              fontSize: 12.5,
              fontWeight: FontWeight.w500,
              color: textColor,
              letterSpacing: -0.1,
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color = appearance.itemColor.withValues(alpha: 0.78);
    final side = compact ? 20.0 : 22.0;
    final iconSize = compact ? 14.0 : 16.0;
    final itemIconColor = colorScheme.onSurfaceVariant.withValues(alpha: 0.85);
    final itemTextColor = colorScheme.onSurface.withValues(alpha: 0.92);
    final dangerColor = colorScheme.error.withValues(alpha: 0.90);

    return SizedBox(
      width: side,
      height: side,
      child: PopupMenuButton<_GroupQuickAction>(
        tooltip: '分组操作',
        padding: EdgeInsets.zero,
        borderRadius: BorderRadius.circular(6),
        color: Color.alphaBlend(
          colorScheme.surfaceContainerHighest.withValues(alpha: 0.75),
          colorScheme.surface,
        ),
        elevation: 6,
        shadowColor: Colors.black.withValues(alpha: 0.45),
        offset: const Offset(0, 4),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(10),
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.4),
            width: 1,
          ),
        ),
        constraints: const BoxConstraints(minWidth: 118, maxWidth: 138),
        menuPadding: const EdgeInsets.symmetric(horizontal: 4, vertical: 4),
        onOpened: () => onMenuOpenChanged?.call(true),
        onCanceled: () => onMenuOpenChanged?.call(false),
        child: Center(
          child: Icon(Icons.more_horiz_rounded, size: iconSize, color: color),
        ),
        onSelected: (action) {
          onMenuOpenChanged?.call(false);
          switch (action) {
            case _GroupQuickAction.createChat:
              onCreateChat();
            case _GroupQuickAction.rename:
              onRename();
            case _GroupQuickAction.togglePinned:
              onTogglePinned();
            case _GroupQuickAction.delete:
              onDelete();
          }
        },
        itemBuilder: (context) => <PopupMenuEntry<_GroupQuickAction>>[
          _menuItem(
            value: _GroupQuickAction.createChat,
            icon: Icons.add_comment_outlined,
            label: '新建对话',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          _menuItem(
            value: _GroupQuickAction.rename,
            icon: Icons.edit_outlined,
            label: '编辑名称',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          _menuItem(
            value: _GroupQuickAction.togglePinned,
            icon: pinned ? Icons.push_pin_outlined : Icons.push_pin_rounded,
            label: pinned ? '取消置顶' : '置顶',
            iconColor: itemIconColor,
            textColor: itemTextColor,
          ),
          const PopupMenuDivider(height: 8),
          _menuItem(
            value: _GroupQuickAction.delete,
            icon: Icons.delete_outline_rounded,
            label: '删除',
            iconColor: dangerColor,
            textColor: dangerColor,
          ),
        ],
      ),
    );
  }
}
