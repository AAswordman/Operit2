// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/StreamMarkdownRendererState.dart';
import '../../part/ThinkToolsXmlNodeGrouper.dart';
import '../../../viewmodel/ChatViewModel.dart';

class AiMessageComposable extends StatefulWidget {
  const AiMessageComposable({
    super.key,
    required this.message,
    required this.isStreaming,
  });

  final ChatUiMessage message;
  final bool isStreaming;

  @override
  State<AiMessageComposable> createState() => _AiMessageComposableState();
}

class _AiMessageComposableState extends State<AiMessageComposable> {
  late StreamMarkdownRendererState _rendererState;
  late String _messageKey;

  @override
  void initState() {
    super.initState();
    _messageKey = widget.message.stableKey;
    _rendererState = StreamMarkdownRendererState();
  }

  @override
  void didUpdateWidget(covariant AiMessageComposable oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.message.stableKey != _messageKey) {
      _messageKey = widget.message.stableKey;
      _rendererState = StreamMarkdownRendererState();
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final detailText = _detailText(widget.message);
    const nodeGrouper = ThinkToolsXmlNodeGrouper(showThinkingProcess: true);

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
              content: widget.message.content,
              contentStream: widget.message.contentStream,
              isStreaming: widget.isStreaming,
              textColor: colorScheme.onSurface,
              backgroundColor: colorScheme.surface,
              nodeGrouper: nodeGrouper,
              rendererId: 'ai-message-${widget.message.stableKey}',
              state: _rendererState,
            ),
          ),
        ],
      ),
    );
  }
}

String _detailText(ChatUiMessage message) {
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
