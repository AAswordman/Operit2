// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

enum ConversationAction {
  rename,
  moveUp,
  moveDown,
  togglePinned,
  toggleLocked,
  delete,
}

class CreateGroupDialog extends StatefulWidget {
  const CreateGroupDialog({super.key});

  @override
  State<CreateGroupDialog> createState() => _CreateGroupDialogState();
}

class _CreateGroupDialogState extends State<CreateGroupDialog> {
  final TextEditingController _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('新建分组'),
      content: TextField(
        controller: _controller,
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
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: const Text('创建'),
        ),
      ],
    );
  }
}

class RenameConversationDialog extends StatefulWidget {
  const RenameConversationDialog({super.key, required this.history});

  final core_proxy.ChatHistory history;

  @override
  State<RenameConversationDialog> createState() =>
      _RenameConversationDialogState();
}

class _RenameConversationDialogState extends State<RenameConversationDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.history.title,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('编辑标题'),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: const InputDecoration(labelText: '新标题'),
        textInputAction: TextInputAction.done,
        onSubmitted: (value) => Navigator.of(context).pop(value),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: const Text('保存'),
        ),
      ],
    );
  }
}

class DeleteConversationDialog extends StatelessWidget {
  const DeleteConversationDialog({super.key, required this.history});

  final core_proxy.ChatHistory history;

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('确认删除对话'),
      content: Text('删除 “${history.title}”？'),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(true),
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          child: const Text('删除'),
        ),
      ],
    );
  }
}

class ConversationActionDialog extends StatelessWidget {
  const ConversationActionDialog({
    super.key,
    required this.history,
    required this.canMoveUp,
    required this.canMoveDown,
  });

  final core_proxy.ChatHistory history;
  final bool canMoveUp;
  final bool canMoveDown;

  @override
  Widget build(BuildContext context) {
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420),
        child: Card(
          margin: EdgeInsets.zero,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 24),
                  child: Column(
                    children: <Widget>[
                      Text(
                        '聊天记录',
                        style: Theme.of(context).textTheme.headlineSmall
                            ?.copyWith(fontWeight: FontWeight.w700),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        history.title,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: Theme.of(context).textTheme.titleMedium
                            ?.copyWith(
                              color: Theme.of(
                                context,
                              ).colorScheme.onSurfaceVariant,
                            ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 12),
                _ConversationActionTile(
                  icon: Icons.edit,
                  label: '编辑标题',
                  onTap: () =>
                      Navigator.of(context).pop(ConversationAction.rename),
                ),
                _ConversationActionTile(
                  icon: Icons.keyboard_arrow_up,
                  label: '上移',
                  onTap: canMoveUp
                      ? () =>
                            Navigator.of(context).pop(ConversationAction.moveUp)
                      : null,
                ),
                _ConversationActionTile(
                  icon: Icons.keyboard_arrow_down,
                  label: '下移',
                  onTap: canMoveDown
                      ? () => Navigator.of(
                          context,
                        ).pop(ConversationAction.moveDown)
                      : null,
                ),
                _ConversationActionTile(
                  icon: Icons.push_pin,
                  label: history.pinned ? '取消置顶' : '置顶',
                  onTap: () => Navigator.of(
                    context,
                  ).pop(ConversationAction.togglePinned),
                ),
                _ConversationActionTile(
                  icon: history.locked ? Icons.lock_open : Icons.lock,
                  label: history.locked ? '解锁' : '锁定',
                  onTap: () => Navigator.of(
                    context,
                  ).pop(ConversationAction.toggleLocked),
                ),
                _ConversationActionTile(
                  icon: Icons.delete_outline,
                  label: '删除',
                  danger: true,
                  onTap: () =>
                      Navigator.of(context).pop(ConversationAction.delete),
                ),
                Align(
                  alignment: AlignmentDirectional.centerEnd,
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 16),
                    child: TextButton(
                      onPressed: () => Navigator.of(context).pop(),
                      child: const Text('取消'),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ConversationActionTile extends StatelessWidget {
  const _ConversationActionTile({
    required this.icon,
    required this.label,
    required this.onTap,
    this.danger = false,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;
  final bool danger;

  @override
  Widget build(BuildContext context) {
    final color = danger
        ? Theme.of(context).colorScheme.error
        : Theme.of(context).colorScheme.primary;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Material(
        color: danger
            ? Theme.of(
                context,
              ).colorScheme.errorContainer.withValues(alpha: 0.5)
            : Theme.of(
                context,
              ).colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(12),
        child: ListTile(
          enabled: onTap != null,
          dense: true,
          leading: Icon(icon, color: color),
          title: Text(label),
          onTap: onTap,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
        ),
      ),
    );
  }
}
