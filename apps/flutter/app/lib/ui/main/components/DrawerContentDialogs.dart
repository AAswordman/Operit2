// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../l10n/generated/app_localizations.dart';

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

  final core_proxy.ChatHistory history;

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
    required this.canMoveUp,
    required this.canMoveDown,
  });

  final core_proxy.ChatHistory history;
  final bool canMoveUp;
  final bool canMoveDown;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
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
                        l10n.chatHistory,
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
                  label: l10n.editTitle,
                  onTap: () =>
                      Navigator.of(context).pop(ConversationAction.rename),
                ),
                _ConversationActionTile(
                  icon: Icons.keyboard_arrow_up,
                  label: l10n.moveUp,
                  onTap: canMoveUp
                      ? () =>
                            Navigator.of(context).pop(ConversationAction.moveUp)
                      : null,
                ),
                _ConversationActionTile(
                  icon: Icons.keyboard_arrow_down,
                  label: l10n.moveDown,
                  onTap: canMoveDown
                      ? () => Navigator.of(
                          context,
                        ).pop(ConversationAction.moveDown)
                      : null,
                ),
                _ConversationActionTile(
                  icon: Icons.push_pin,
                  label: history.pinned ? l10n.unpin : l10n.pin,
                  onTap: () => Navigator.of(
                    context,
                  ).pop(ConversationAction.togglePinned),
                ),
                _ConversationActionTile(
                  icon: history.locked ? Icons.lock_open : Icons.lock,
                  label: history.locked ? l10n.unlock : l10n.lock,
                  onTap: () => Navigator.of(
                    context,
                  ).pop(ConversationAction.toggleLocked),
                ),
                _ConversationActionTile(
                  icon: Icons.delete_outline,
                  label: l10n.delete,
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
                      child: Text(l10n.cancel),
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
