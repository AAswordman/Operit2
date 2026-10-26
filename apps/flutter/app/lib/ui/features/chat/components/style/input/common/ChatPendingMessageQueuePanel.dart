// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitGlassSurface.dart';
import '../../../../../../theme/OperitTheme.dart';
import 'PendingQueueMessageItem.dart';

/// Displays the shared pending queue above either chat input style.
class ChatPendingMessageQueuePanel extends StatelessWidget {
  /// Creates the queue with host-owned message actions.
  const ChatPendingMessageQueuePanel({
    super.key,
    required this.queuedMessages,
    required this.expanded,
    required this.onExpandedChange,
    required this.onDeleteMessage,
    required this.onEditMessage,
    required this.onSendMessage,
  });

  final List<PendingQueueMessageItem> queuedMessages;
  final bool expanded;
  final ValueChanged<bool>? onExpandedChange;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;

  /// Aligns the themed queue with the composer and plugin panel edges.
  @override
  Widget build(BuildContext context) {
    if (queuedMessages.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final floating = OperitTheme.of(
      context,
    ).themePreferenceSnapshot.chatInputFloating;
    return Padding(
      key: const ValueKey("chat.pending-queue.inset"),
      padding: EdgeInsets.fromLTRB(floating ? 8 : 0, 0, floating ? 8 : 0, 8),
      child: OperitGlassSurface(
        key: const ValueKey("chat.pending-queue.surface"),
        color: colorScheme.surfaceContainer,
        borderRadius: BorderRadius.circular(8),
        material: true,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            InkWell(
              key: const ValueKey("chat.pending-queue.header"),
              onTap: onExpandedChange == null
                  ? null
                  : () => onExpandedChange!(!expanded),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 4,
                ),
                child: Row(
                  children: <Widget>[
                    Icon(
                      Icons.schedule,
                      size: 16,
                      color: colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 4),
                    Expanded(
                      child: Text(
                        l10n.chatPendingQueueTitle(queuedMessages.length),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                    SizedBox(
                      width: 32,
                      height: 32,
                      child: Icon(
                        expanded
                            ? Icons.keyboard_arrow_up
                            : Icons.keyboard_arrow_down,
                        size: 20,
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            if (expanded)
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 220),
                child: ListView.separated(
                  padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                  shrinkWrap: true,
                  itemCount: queuedMessages.length,
                  separatorBuilder: (_, _) => const SizedBox(height: 8),
                  itemBuilder: (context, index) {
                    final item = queuedMessages[index];
                    return _PendingQueueMessageTile(
                      item: item,
                      onDeleteMessage: onDeleteMessage,
                      onEditMessage: onEditMessage,
                      onSendMessage: onSendMessage,
                    );
                  },
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _PendingQueueMessageTile extends StatelessWidget {
  /// Creates a message row with its existing queue actions.
  const _PendingQueueMessageTile({
    required this.item,
    required this.onDeleteMessage,
    required this.onEditMessage,
    required this.onSendMessage,
  });

  final PendingQueueMessageItem item;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;

  /// Builds one pending queue message row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final actionColor = colorScheme.onSurfaceVariant.withValues(alpha: 0.9);
    return OperitGlassSurface(
      color: colorScheme.surface,
      layer: OperitGlassSurfaceLayer.control,
      borderRadius: BorderRadius.circular(8),
      material: true,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Row(
          children: <Widget>[
            _QueueIconAction(
              icon: Icons.edit,
              tooltip: l10n.edit,
              color: actionColor,
              onTap: onEditMessage == null
                  ? null
                  : () => onEditMessage!(item.id),
            ),
            Expanded(
              child: Text(
                item.text,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurface,
                ),
              ),
            ),
            _QueueIconAction(
              icon: Icons.send,
              tooltip: l10n.send,
              color: actionColor,
              onTap: onSendMessage == null
                  ? null
                  : () => onSendMessage!(item.id),
            ),
            _QueueIconAction(
              icon: Icons.delete,
              tooltip: l10n.delete,
              color: actionColor,
              onTap: onDeleteMessage == null
                  ? null
                  : () => onDeleteMessage!(item.id),
            ),
          ],
        ),
      ),
    );
  }
}

class _QueueIconAction extends StatelessWidget {
  /// Creates a compact message action.
  const _QueueIconAction({
    required this.icon,
    required this.tooltip,
    required this.color,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final Color color;
  final VoidCallback? onTap;

  /// Builds one icon action in the pending queue.
  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 16,
        child: SizedBox(
          width: 28,
          height: 28,
          child: Icon(icon, size: 16, color: color),
        ),
      ),
    );
  }
}
