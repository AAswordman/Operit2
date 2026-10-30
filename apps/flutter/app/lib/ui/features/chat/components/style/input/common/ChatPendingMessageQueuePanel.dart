// ignore_for_file: file_names

import 'dart:async';
import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitTheme.dart';
import 'PendingQueueMessageItem.dart';

/// Displays the shared pending queue as a folder-ear tab / drawer above chat input.
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
    this.onGuideMessage,
    this.canGuide = false,
  });

  final List<PendingQueueMessageItem> queuedMessages;
  final bool expanded;
  final ValueChanged<bool>? onExpandedChange;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;
  final ValueChanged<int>? onGuideMessage;
  final bool canGuide;

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

    // Collapsed: Compact folder-ear tab sticking out above input box (Image #1)
    final tabContent = Align(
      alignment: Alignment.centerLeft,
      child: Padding(
        padding: EdgeInsets.only(left: floating ? 22 : 12),
        child: Material(
          key: const ValueKey("chat.pending-queue.surface"),
          color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.85),
          shape: RoundedRectangleBorder(
            borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
            side: BorderSide(
              color: colorScheme.outlineVariant.withValues(alpha: 0.25),
              width: 0.8,
            ),
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            key: const ValueKey("chat.pending-queue.header"),
            onTap: onExpandedChange == null
                ? null
                : () => onExpandedChange!(true),
            child: Padding(
              padding: const EdgeInsets.symmetric(
                horizontal: 10,
                vertical: 4,
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(
                    Icons.schedule_rounded,
                    size: 13,
                    color: colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
                  ),
                  const SizedBox(width: 5),
                  Text(
                    l10n.chatPendingQueueTitle(queuedMessages.length),
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w500,
                      fontSize: 11.5,
                    ),
                  ),
                  const SizedBox(width: 3),
                  Icon(
                    Icons.keyboard_arrow_up_rounded,
                    size: 15,
                    color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );

    // Expanded: Full-width attached drawer shelf directly joined to the input box
    final drawerContent = Padding(
      padding: EdgeInsets.symmetric(horizontal: floating ? 8 : 0),
      child: Material(
        key: const ValueKey("chat.pending-queue.surface"),
        color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.95),
        shape: RoundedRectangleBorder(
          borderRadius: const BorderRadius.vertical(top: Radius.circular(14)),
          side: BorderSide(
            color: colorScheme.outlineVariant.withValues(alpha: 0.25),
            width: 0.8,
          ),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            InkWell(
              key: const ValueKey("chat.pending-queue.header"),
              onTap: onExpandedChange == null
                  ? null
                  : () => onExpandedChange!(false),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 6, 10, 5),
                child: Row(
                  children: <Widget>[
                    Icon(
                      Icons.schedule_rounded,
                      size: 13,
                      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.8),
                    ),
                    const SizedBox(width: 5),
                    Expanded(
                      child: Text(
                        l10n.chatPendingQueueTitle(queuedMessages.length),
                        style: theme.textTheme.labelMedium?.copyWith(
                          color: colorScheme.onSurface,
                          fontWeight: FontWeight.w600,
                          fontSize: 11.5,
                        ),
                      ),
                    ),
                    Icon(
                      Icons.keyboard_arrow_down_rounded,
                      size: 16,
                      color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                    ),
                  ],
                ),
              ),
            ),
            Divider(
              height: 1,
              thickness: 0.6,
              color: colorScheme.outlineVariant.withValues(alpha: 0.2),
            ),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 200),
              child: ListView.separated(
                padding: const EdgeInsets.fromLTRB(8, 6, 8, 12),
                shrinkWrap: true,
                itemCount: queuedMessages.length,
                separatorBuilder: (_, _) => const SizedBox(height: 4),
                itemBuilder: (context, index) {
                  final item = queuedMessages[index];
                  return _PendingQueueMessageTile(
                    item: item,
                    onDeleteMessage: onDeleteMessage,
                    onEditMessage: onEditMessage,
                    onSendMessage: onSendMessage,
                    onGuideMessage: canGuide ? onGuideMessage : null,
                  );
                },
              ),
            ),
          ],
        ),
      ),
    );

    return Padding(
      key: const ValueKey("chat.pending-queue.inset"),
      padding: EdgeInsets.zero,
      child: Transform.translate(
        offset: const Offset(0, 10),
        child: expanded ? drawerContent : tabContent,
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
    this.onGuideMessage,
  });

  final PendingQueueMessageItem item;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;
  final ValueChanged<int>? onGuideMessage;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final defaultIconColor = colorScheme.onSurfaceVariant.withValues(alpha: 0.7);

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surface.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(6),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
          width: 0.6,
        ),
      ),
      padding: const EdgeInsets.fromLTRB(8, 4, 6, 4),
      child: Row(
        children: <Widget>[
          _QueueIconAction(
            icon: Icons.edit_note_rounded,
            tooltip: l10n.chatQueueWithdrawToEditor,
            color: defaultIconColor,
            onTap: onEditMessage == null
                ? null
                : () => onEditMessage!(item.id),
          ),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              item.text,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurface.withValues(alpha: 0.88),
                fontSize: 11.5,
                height: 1.35,
              ),
            ),
          ),
          const SizedBox(width: 6),
          if (onGuideMessage != null)
            _QueueIconAction(
              icon: Icons.call_split_rounded,
              tooltip: l10n.chatGuideCurrentTurn,
              color: defaultIconColor,
              hoverColor: colorScheme.primary,
              onTap: () => onGuideMessage!(item.id),
            ),
          _QueueIconAction(
            icon: Icons.arrow_upward_rounded,
            tooltip: l10n.chatInterruptAndSend,
            color: defaultIconColor,
            hoverColor: colorScheme.primary,
            onTap: onSendMessage == null
                ? null
                : () => onSendMessage!(item.id),
          ),
          _QueueIconAction(
            icon: Icons.close_rounded,
            tooltip: l10n.delete,
            color: defaultIconColor,
            hoverColor: colorScheme.error,
            onTap: onDeleteMessage == null
                ? null
                : () => onDeleteMessage!(item.id),
          ),
        ],
      ),
    );
  }
}

class _QueueIconAction extends StatefulWidget {
  /// Creates a compact message action.
  const _QueueIconAction({
    required this.icon,
    required this.tooltip,
    required this.color,
    this.hoverColor,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final Color color;
  final Color? hoverColor;
  final VoidCallback? onTap;

  @override
  State<_QueueIconAction> createState() => _QueueIconActionState();
}

class _QueueIconActionState extends State<_QueueIconAction> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final activeColor = _isHovered
        ? (widget.hoverColor ?? theme.colorScheme.onSurface)
        : widget.color;

    return Tooltip(
      message: widget.tooltip,
      child: MouseRegion(
        onEnter: (_) => setState(() => _isHovered = true),
        onExit: (_) => setState(() => _isHovered = false),
        child: InkResponse(
          onTap: widget.onTap,
          radius: 13,
          child: Container(
            width: 24,
            height: 24,
            decoration: BoxDecoration(
              color: _isHovered
                  ? theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.4)
                  : Colors.transparent,
              borderRadius: BorderRadius.circular(5),
            ),
            child: Center(
              child: Icon(
                widget.icon,
                size: 14,
                color: activeColor,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
