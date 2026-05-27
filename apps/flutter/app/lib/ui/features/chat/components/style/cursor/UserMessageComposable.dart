// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../core/chat/OperitChatRuntime.dart';

class UserMessageComposable extends StatelessWidget {
  const UserMessageComposable({super.key, required this.message});

  final ChatRuntimeMessage message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final textColor = colorScheme.onPrimaryContainer;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: Card(
        margin: EdgeInsets.zero,
        color: colorScheme.primaryContainer,
        elevation: 0,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                'Prompt',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: textColor.withValues(alpha: 0.7),
                ),
              ),
              const SizedBox(height: 8),
              SelectableText(
                _cleanUserContent(message.content),
                style: theme.textTheme.bodyMedium?.copyWith(color: textColor),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

String _cleanUserContent(String content) {
  return content
      .replaceAll(
        RegExp(r'<memory\b[^>]*>[\s\S]*?</memory>', caseSensitive: false),
        '',
      )
      .replaceAll(
        RegExp(
          r'<proxy_sender\b[^>]*>[\s\S]*?</proxy_sender>',
          caseSensitive: false,
        ),
        '',
      )
      .trim();
}
