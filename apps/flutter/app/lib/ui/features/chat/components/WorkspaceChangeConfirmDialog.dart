// ignore_for_file: file_names

import 'package:flutter/material.dart';

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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final title = switch (mode) {
      WorkspaceChangeConfirmMode.rollback => '作業フォルダーを元に戻す',
      WorkspaceChangeConfirmMode.editAndResend => '編集して再送信',
    };
    final message = switch (mode) {
      WorkspaceChangeConfirmMode.rollback =>
        '作業フォルダーをこのメッセージより前の状態に戻し、このメッセージ以降の会話を削除します。',
      WorkspaceChangeConfirmMode.editAndResend =>
        '作業フォルダーをこのメッセージより前の状態に戻し、編集した内容を再送信します。',
    };
    final confirmText = switch (mode) {
      WorkspaceChangeConfirmMode.rollback => '元に戻す',
      WorkspaceChangeConfirmMode.editAndResend => '保存して再送信',
    };

    return AlertDialog(
      title: Text(title),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520, maxHeight: 420),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(message),
            const SizedBox(height: 12),
            if (changes.isEmpty)
              Text(
                '作業フォルダー内のファイル変更はありません。',
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
                        '${change.changeType} · ${change.changedLines}行',
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
          child: const Text('キャンセル'),
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
