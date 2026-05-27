// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../core/chat/OperitChatRuntime.dart';
import '../../../../../common/markdown/StreamMarkdownRenderer.dart';

class AiMessageComposable extends StatelessWidget {
  const AiMessageComposable({
    super.key,
    required this.message,
    required this.isStreaming,
  });

  final ChatRuntimeMessage message;
  final bool isStreaming;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final detailText = _detailText(message);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
            child: Row(
              children: <Widget>[
                Text(
                  'Response',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurface.withValues(alpha: 0.7),
                  ),
                ),
                if (detailText.isNotEmpty) ...<Widget>[
                  const Spacer(),
                  Text(
                    detailText,
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: colorScheme.onSurface.withValues(alpha: 0.5),
                    ),
                  ),
                ],
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: StreamMarkdownRenderer(
              content: message.content,
              isStreaming: isStreaming,
              streamState: isStreaming ? message.markdownStreamState : null,
              textColor: colorScheme.onSurface,
              backgroundColor: colorScheme.surface,
            ),
          ),
        ],
      ),
    );
  }
}

String _detailText(ChatRuntimeMessage message) {
  final parts = <String>[];
  if (message.roleName.isNotEmpty) {
    parts.add(message.roleName);
  }
  if (message.modelName.isNotEmpty && message.provider.isNotEmpty) {
    parts.add('${message.modelName} by ${message.provider}');
  } else if (message.modelName.isNotEmpty) {
    parts.add(message.modelName);
  } else if (message.provider.isNotEmpty) {
    parts.add(message.provider);
  }
  return parts.join(' | ');
}
