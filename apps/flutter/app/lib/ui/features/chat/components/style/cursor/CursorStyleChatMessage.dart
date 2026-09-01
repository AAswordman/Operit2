// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../viewmodel/ChatViewModel.dart';
import 'AiMessageComposable.dart';
import '../SummaryMessageComposable.dart';
import 'UserMessageComposable.dart';

class CursorStyleChatMessage extends StatelessWidget {
  const CursorStyleChatMessage({
    super.key,
    required this.message,
    required this.isStreaming,
    this.currentCharacterCardAvatarUri,
    this.splitMarkdownContent,
    this.onDeleteMessage,
    this.onEditSummary,
    this.enableDialogs = true,
  });

  final ChatUiMessage message;
  final bool isStreaming;
  final String? currentCharacterCardAvatarUri;
  final MarkdownContentSplitter? splitMarkdownContent;
  final Future<void> Function(int timestamp)? onDeleteMessage;
  final ValueChanged<ChatUiMessage>? onEditSummary;
  final bool enableDialogs;

  @override
  Widget build(BuildContext context) {
    switch (message.sender) {
      case 'user':
        return UserMessageComposable(message: message, useBubbleStyle: false);
      case 'ai':
        return AiMessageComposable(
          message: message,
          isStreaming: isStreaming,
          useBubbleStyle: false,
          avatarImagePath: currentCharacterCardAvatarUri,
          splitMarkdownContent: splitMarkdownContent,
        );
      case 'summary':
        return SummaryMessageComposable(
          message: message,
          onDelete: onDeleteMessage == null
              ? null
              : () => onDeleteMessage!(message.timestamp),
          onEdit: onEditSummary,
          enableDialog: enableDialogs,
        );
    }
    return _SystemMessageComposable(message: message);
  }
}

class _SystemMessageComposable extends StatelessWidget {
  const _SystemMessageComposable({required this.message});

  final ChatUiMessage message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: SelectableText(
        message.displayText,
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
