// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../viewmodel/ChatViewModel.dart';

enum WorkspaceChangeConfirmMode { rollback, editAndResend }

class WorkspaceChangeConfirmDialog extends StatelessWidget {
  const WorkspaceChangeConfirmDialog({
    super.key,
    required this.mode,
    required this.changes,
    required this.onConfirm,
  });

  final WorkspaceChangeConfirmMode mode;
  final List<WorkspaceFileChange> changes;
  final Future<void> Function() onConfirm;

  /// Builds a bounded confirmation dialog with a scrollable file change list.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final title = switch (mode) {
      WorkspaceChangeConfirmMode.rollback => l10n.chatRollbackConfirmTitle,
      WorkspaceChangeConfirmMode.editAndResend => l10n.chatResendConfirmTitle,
    };
    final message = switch (mode) {
      WorkspaceChangeConfirmMode.rollback => l10n.chatRollbackConfirmMessage,
      WorkspaceChangeConfirmMode.editAndResend =>
        l10n.chatResendConfirmMessage,
    };
    final confirmText = switch (mode) {
      WorkspaceChangeConfirmMode.rollback => l10n.chatRollbackConfirm,
      WorkspaceChangeConfirmMode.editAndResend => l10n.chatSaveResend,
    };

    return AlertDialog(
      title: Text(title),
      content: ConstrainedBox(
        // A tight width keeps AlertDialog's intrinsic sizing out of the lazy list.
        constraints: const BoxConstraints(
          minWidth: 520,
          maxWidth: 520,
          maxHeight: 420,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(message),
            const SizedBox(height: 12),
            if (changes.isEmpty)
              Text(
                l10n.chatNoWorkspaceChanges,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              )
            else
              Flexible(
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: changes.length,
                  separatorBuilder: (context, index) =>
                      const Divider(height: 1),
                  itemBuilder: (context, index) {
                    final change = changes[index];
                    return ListTile(
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                      leading: Icon(
                        _iconForChange(change.changeType),
                        size: 18,
                        color: colorScheme.primary,
                      ),
                      title: Text(
                        change.path,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                      subtitle: Text(
                        l10n.chatWorkspaceChangeLines(
                          change.changeType.toString(),
                          change.changedLines,
                        ),
                      ),
                    );
                  },
                ),
              ),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () async {
            await onConfirm();
            if (context.mounted) {
              Navigator.of(context).pop();
            }
          },
          child: Text(confirmText),
        ),
      ],
    );
  }
}

IconData _iconForChange(Object? changeType) {
  final type = changeType.toString().toLowerCase();
  if (type.contains('delete')) {
    return Icons.delete_outline;
  }
  if (type.contains('create') || type.contains('add')) {
    return Icons.add_circle_outline;
  }
  return Icons.edit_outlined;
}
