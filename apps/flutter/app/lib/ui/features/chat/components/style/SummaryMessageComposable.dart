// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/ChatViewModel.dart';

/// Renders a summary as a compact divider and opens its complete text on tap.
class SummaryMessageComposable extends StatelessWidget {
  const SummaryMessageComposable({
    super.key,
    required this.message,
    this.onDelete,
    this.onEdit,
    this.enableDialog = true,
  });

  final ChatUiMessage message;
  final VoidCallback? onDelete;
  final ValueChanged<ChatUiMessage>? onEdit;
  final bool enableDialog;

  /// Builds the compact summary divider used in both chat styles.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: InkWell(
        onTap: enableDialog ? () => _showSummaryDialog(context, l10n) : null,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 2),
          child: Row(
            children: <Widget>[
              Expanded(
                child: Divider(
                  color: theme.colorScheme.primary.withValues(alpha: 0.5),
                ),
              ),
              DecoratedBox(
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 2,
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      Icon(
                        Icons.info_outline,
                        size: 14,
                        color: theme.colorScheme.primary,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        l10n.historyDialogSummary,
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: theme.colorScheme.primary,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              Expanded(
                child: Divider(
                  color: theme.colorScheme.primary.withValues(alpha: 0.5),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Displays the full summary in a bounded, scrollable dialog.
  void _showSummaryDialog(BuildContext context, AppLocalizations l10n) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) {
        final theme = Theme.of(dialogContext);
        return AlertDialog(
          title: Text(l10n.historyDialogSummary, textAlign: TextAlign.center),
          content: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 400),
            child: SingleChildScrollView(
              child: SelectableText(
                message.displayText,
                style: theme.textTheme.bodyMedium,
              ),
            ),
          ),
          actions: <Widget>[
            if (onEdit != null)
              TextButton(
                onPressed: () {
                  Navigator.of(dialogContext).pop();
                  onEdit!(message);
                },
                child: Text(l10n.edit),
              ),
            if (onDelete != null)
              TextButton(
                onPressed: () {
                  Navigator.of(dialogContext).pop();
                  _showDeleteConfirmation(context, l10n);
                },
                child: Text(
                  l10n.delete,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: Text(
                MaterialLocalizations.of(dialogContext).closeButtonLabel,
              ),
            ),
          ],
        );
      },
    );
  }

  /// Confirms deletion before removing the persisted summary message.
  void _showDeleteConfirmation(BuildContext context, AppLocalizations l10n) {
    showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: Text(l10n.confirmDeleteSummary),
        content: Text(l10n.confirmDeleteSummaryMessage),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: () {
              Navigator.of(dialogContext).pop();
              onDelete?.call();
            },
            child: Text(l10n.delete),
          ),
        ],
      ),
    );
  }
}
