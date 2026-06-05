// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../viewmodel/ChatViewModel.dart';
import 'AiMessageComposable.dart';
import 'UserMessageComposable.dart';

class CursorStyleChatMessage extends StatelessWidget {
  const CursorStyleChatMessage({
    super.key,
    required this.message,
    required this.isStreaming,
  });

  final ChatUiMessage message;
  final bool isStreaming;

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
        );
      case 'summary':
        return _SummaryMessageComposable(message: message);
    }
    return _SystemMessageComposable(message: message);
  }
}

class _SummaryMessageComposable extends StatelessWidget {
  const _SummaryMessageComposable({required this.message});

  final ChatUiMessage message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      padding: const EdgeInsets.fromLTRB(12, 10, 12, 10),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        message.content,
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
          height: 1.4,
        ),
      ),
    );
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
        message.content,
        style: theme.textTheme.bodySmall?.copyWith(
          color: theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}
