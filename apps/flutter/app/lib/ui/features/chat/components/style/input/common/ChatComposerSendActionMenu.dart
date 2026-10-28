// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

/// Delivery choices for a draft submitted while the current turn is still busy.
enum ChatComposerSendAction { queue, guide, sendNow }

/// Keeps one-tap queueing on the primary action and exposes explicit alternatives.
class ChatComposerSendActionMenu extends StatelessWidget {
  const ChatComposerSendActionMenu({
    super.key,
    required this.onAction,
    this.targetSize = 30,
    this.iconSize = 20,
  });

  final ValueChanged<ChatComposerSendAction> onAction;
  final double targetSize;
  final double iconSize;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return PopupMenuButton<ChatComposerSendAction>(
      key: const ValueKey<String>('chat.send-options'),
      tooltip: l10n.chatSendOptions,
      padding: EdgeInsets.zero,
      constraints: const BoxConstraints(minWidth: 280, maxWidth: 340),
      onSelected: onAction,
      itemBuilder: (context) => <PopupMenuEntry<ChatComposerSendAction>>[
        _SendActionMenuItem(
          value: ChatComposerSendAction.queue,
          itemKey: const ValueKey<String>('chat.send-option.queue'),
          icon: Icons.schedule,
          title: l10n.chatQueueAddMessage,
          description: l10n.chatQueueAddMessageDescription,
          color: colorScheme.onSurfaceVariant,
        ),
        _SendActionMenuItem(
          value: ChatComposerSendAction.guide,
          itemKey: const ValueKey<String>('chat.send-option.guide'),
          icon: Icons.alt_route,
          title: l10n.chatGuideCurrentTurn,
          description: l10n.chatGuideCurrentTurnDescription,
          color: colorScheme.primary,
        ),
        _SendActionMenuItem(
          value: ChatComposerSendAction.sendNow,
          itemKey: const ValueKey<String>('chat.send-option.send-now'),
          icon: Icons.send,
          title: l10n.chatInterruptAndSend,
          description: l10n.chatInterruptAndSendDescription,
          color: colorScheme.error,
        ),
      ],
      child: SizedBox(
        width: targetSize,
        height: targetSize,
        child: Icon(
          Icons.expand_more,
          size: iconSize,
          color: colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _SendActionMenuItem extends PopupMenuItem<ChatComposerSendAction> {
  _SendActionMenuItem({
    required super.value,
    required Key itemKey,
    required IconData icon,
    required String title,
    required String description,
    required Color color,
  }) : super(
         key: itemKey,
         height: 68,
         child: Row(
           crossAxisAlignment: CrossAxisAlignment.start,
           children: <Widget>[
             Padding(
               padding: const EdgeInsets.only(top: 2),
               child: Icon(icon, size: 20, color: color),
             ),
             const SizedBox(width: 12),
             Expanded(
               child: Column(
                 mainAxisSize: MainAxisSize.min,
                 crossAxisAlignment: CrossAxisAlignment.start,
                 children: <Widget>[
                   Text(
                     title,
                     maxLines: 1,
                     overflow: TextOverflow.ellipsis,
                     style: TextStyle(
                       color: color,
                       fontWeight: FontWeight.w500,
                     ),
                   ),
                   const SizedBox(height: 3),
                   Text(
                     description,
                     maxLines: 2,
                     overflow: TextOverflow.ellipsis,
                     style: TextStyle(
                       color: color.withValues(alpha: 0.78),
                       fontSize: 12,
                       height: 1.25,
                     ),
                   ),
                 ],
               ),
             ),
           ],
         ),
       );
}
