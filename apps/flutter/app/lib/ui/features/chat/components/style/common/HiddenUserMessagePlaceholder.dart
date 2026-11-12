// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../viewmodel/ChatViewModel.dart';

/// Displays an automatic user turn without rendering its source or attachments.
class HiddenUserMessagePlaceholder extends StatelessWidget {
  /// Creates the shared placeholder with optional source inspection.
  const HiddenUserMessagePlaceholder({
    super.key,
    required this.message,
    required this.textColor,
    required this.enableDialogs,
  });

  final ChatUiMessage message;
  final Color textColor;
  final bool enableDialogs;

  /// Builds the same compact marker for both transcript styles.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final dividerColor = Theme.of(
      context,
    ).colorScheme.primary.withValues(alpha: 0.28);
    return Align(
      alignment: Alignment.centerRight,
      heightFactor: 1,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 220),
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: enableDialogs ? () => _showSource(context) : null,
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Row(
              children: <Widget>[
                Expanded(child: Divider(color: dividerColor, thickness: 1)),
                const SizedBox(width: 8),
                Flexible(
                  flex: 8,
                  child: Text(
                    l10n.autoTriggeredUserMessage,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: textColor.withValues(alpha: 0.72),
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(child: Divider(color: dividerColor, thickness: 1)),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Opens the complete stored source without interpreting embedded markup.
  Future<void> _showSource(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(l10n.hiddenUserMessage),
        content: SingleChildScrollView(child: Text(message.displayText)),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(l10n.close),
          ),
        ],
      ),
    );
  }
}
