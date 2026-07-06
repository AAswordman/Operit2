// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';
import '../../common/components/OperitDialog.dart';

enum ConversationAction {
  openInWindow,
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
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.createGroupTitle),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: InputDecoration(labelText: l10n.groupNameLabel),
        onSubmitted: (value) => Navigator.of(context).pop(value),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: Text(l10n.create),
        ),
      ],
    );
  }
}

class RenameConversationDialog extends StatefulWidget {
  const RenameConversationDialog({super.key, required this.history});

  final core_proxy.ChatHistoryListItem history;

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
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.renameConversationTitle),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: InputDecoration(labelText: l10n.newTitleLabel),
        textInputAction: TextInputAction.done,
        onSubmitted: (value) => Navigator.of(context).pop(value),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(_controller.text),
          child: Text(l10n.save),
        ),
      ],
    );
  }
}

class DeleteConversationDialog extends StatelessWidget {
  const DeleteConversationDialog({super.key, required this.history});

  final core_proxy.ChatHistoryListItem history;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.deleteConversationTitle),
      content: Text(l10n.deleteConversationMessage(history.title)),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(l10n.cancel),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(true),
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          child: Text(l10n.delete),
        ),
      ],
    );
  }
}

class ConversationActionDialog extends StatelessWidget {
  const ConversationActionDialog({
    super.key,
    required this.history,
    required this.canOpenInWindow,
    required this.canMoveUp,
    required this.canMoveDown,
  });

  final core_proxy.ChatHistoryListItem history;
  final bool canOpenInWindow;
  final bool canMoveUp;
  final bool canMoveDown;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return OperitDialogScaffold(
      title: l10n.chatHistory,
      maxWidth: 420,
      contentPadding: const EdgeInsets.fromLTRB(8, 12, 8, 8),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
            child: Text(
              history.title,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          if (canOpenInWindow)
            _ConversationActionTile(
              icon: Icons.open_in_new,
              label: '在新窗口打开',
              onTap: () =>
                  Navigator.of(context).pop(ConversationAction.openInWindow),
            ),
          _ConversationActionTile(
            icon: Icons.edit,
            label: l10n.editTitle,
            onTap: () => Navigator.of(context).pop(ConversationAction.rename),
          ),
          _ConversationActionTile(
            icon: Icons.keyboard_arrow_up,
            label: l10n.moveUp,
            onTap: canMoveUp
                ? () => Navigator.of(context).pop(ConversationAction.moveUp)
                : null,
          ),
          _ConversationActionTile(
            icon: Icons.keyboard_arrow_down,
            label: l10n.moveDown,
            onTap: canMoveDown
                ? () => Navigator.of(context).pop(ConversationAction.moveDown)
                : null,
          ),
          _ConversationActionTile(
            icon: Icons.push_pin,
            label: history.pinned ? l10n.unpin : l10n.pin,
            onTap: () =>
                Navigator.of(context).pop(ConversationAction.togglePinned),
          ),
          _ConversationActionTile(
            icon: history.locked ? Icons.lock_open : Icons.lock,
            label: history.locked ? l10n.unlock : l10n.lock,
            onTap: () =>
                Navigator.of(context).pop(ConversationAction.toggleLocked),
          ),
          _ConversationActionTile(
            icon: Icons.delete_outline,
            label: l10n.delete,
            danger: true,
            onTap: () => Navigator.of(context).pop(ConversationAction.delete),
          ),
        ],
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
