// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';

class ChatArea extends StatelessWidget {
  const ChatArea({
    super.key,
    required this.messages,
    required this.isLoading,
    required this.errorMessage,
    required this.scrollController,
  });

  final List<ChatRuntimeMessage> messages;
  final bool isLoading;
  final String? errorMessage;
  final ScrollController scrollController;

  @override
  Widget build(BuildContext context) {
    final showLoadingIndicator = _shouldShowLoadingIndicator();
    final itemCount =
        messages.length +
        (showLoadingIndicator || errorMessage != null ? 1 : 0);

    if (itemCount == 0) {
      return const _EmptyChatArea();
    }

    return ListView.separated(
      controller: scrollController,
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
      itemCount: itemCount,
      separatorBuilder: (context, index) {
        return const SizedBox(height: 8);
      },
      itemBuilder: (context, index) {
        if (index < messages.length) {
          return _MessageItem(message: messages[index]);
        }
        if (errorMessage != null) {
          return _StatusMessage(text: errorMessage!, isError: true);
        }
        return const Padding(
          padding: EdgeInsets.only(left: 16, top: 2, bottom: 2),
          child: LoadingDotsIndicator(),
        );
      },
    );
  }

  bool _shouldShowLoadingIndicator() {
    if (!isLoading || messages.isEmpty) {
      return isLoading && messages.isEmpty;
    }
    final lastMessage = messages.last;
    return lastMessage.sender == 'user' ||
        (lastMessage.sender == 'ai' && lastMessage.content.isEmpty);
  }
}

class _MessageItem extends StatelessWidget {
  const _MessageItem({required this.message});

  final ChatRuntimeMessage message;

  @override
  Widget build(BuildContext context) {
    if (message.sender == 'user') {
      return _UserMessage(message: message);
    }
    if (message.sender == 'ai') {
      return _AiMessage(message: message);
    }
    return _SystemMessage(message: message);
  }
}

class _UserMessage extends StatelessWidget {
  const _UserMessage({required this.message});

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

class _AiMessage extends StatelessWidget {
  const _AiMessage({required this.message});

  final ChatRuntimeMessage message;

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
            child: _AiContent(content: message.content),
          ),
        ],
      ),
    );
  }
}

class _AiContent extends StatelessWidget {
  const _AiContent({required this.content});

  final String content;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final segments = _splitAiSegments(content);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: segments
          .map((segment) {
            switch (segment.kind) {
              case _AiSegmentKind.text:
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: SelectableText(
                    segment.text,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: colorScheme.onSurface,
                      height: 1.45,
                    ),
                  ),
                );
              case _AiSegmentKind.thinking:
                return _XmlLikePanel(
                  label: 'Thinking',
                  text: segment.text,
                  color: colorScheme.secondary,
                );
              case _AiSegmentKind.status:
                return _XmlLikePanel(
                  label: 'Status',
                  text: segment.text,
                  color: colorScheme.tertiary,
                );
              case _AiSegmentKind.tool:
                return _XmlLikePanel(
                  label: 'Tool',
                  text: segment.text,
                  color: colorScheme.primary,
                );
            }
          })
          .toList(growable: false),
    );
  }
}

class _XmlLikePanel extends StatelessWidget {
  const _XmlLikePanel({
    required this.label,
    required this.text,
    required this.color,
  });

  final String label;
  final String text;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.fromLTRB(10, 8, 10, 8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(8),
        border: Border(
          left: BorderSide(color: color.withValues(alpha: 0.55), width: 3),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: theme.textTheme.labelSmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 4),
          SelectableText(
            text,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
              height: 1.35,
            ),
          ),
        ],
      ),
    );
  }
}

class _SystemMessage extends StatelessWidget {
  const _SystemMessage({required this.message});

  final ChatRuntimeMessage message;

  @override
  Widget build(BuildContext context) {
    return _StatusMessage(text: message.content);
  }
}

class _StatusMessage extends StatelessWidget {
  const _StatusMessage({required this.text, this.isError = false});

  final String text;
  final bool isError;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      child: SelectableText(
        text,
        style: theme.textTheme.bodySmall?.copyWith(
          color: isError
              ? theme.colorScheme.error
              : theme.colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class LoadingDotsIndicator extends StatefulWidget {
  const LoadingDotsIndicator({super.key});

  @override
  State<LoadingDotsIndicator> createState() => _LoadingDotsIndicatorState();
}

class _LoadingDotsIndicatorState extends State<LoadingDotsIndicator>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    )..repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(
      context,
    ).colorScheme.onSurface.withValues(alpha: 0.6);
    return AnimatedBuilder(
      animation: _controller,
      builder: (context, child) {
        return Row(
          mainAxisSize: MainAxisSize.min,
          children: List<Widget>.generate(3, (index) {
            final progress = (_controller.value + index * 0.18) % 1;
            final jump = progress < 0.5 ? progress * 2 : (1 - progress) * 2;
            return Transform.translate(
              offset: Offset(0, -5 * jump),
              child: Container(
                width: 6,
                height: 6,
                margin: const EdgeInsets.symmetric(horizontal: 3),
                decoration: BoxDecoration(color: color, shape: BoxShape.circle),
              ),
            );
          }),
        );
      },
    );
  }
}

class _EmptyChatArea extends StatelessWidget {
  const _EmptyChatArea();

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Text(
        'Operit',
        style: theme.textTheme.displaySmall?.copyWith(
          color: theme.colorScheme.primary.withValues(alpha: 0.38),
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

enum _AiSegmentKind { text, thinking, status, tool }

class _AiSegment {
  const _AiSegment({required this.kind, required this.text});

  final _AiSegmentKind kind;
  final String text;
}

List<_AiSegment> _splitAiSegments(String content) {
  final segments = <_AiSegment>[];
  final pattern = RegExp(
    r'<(think|thinking|status|tool|tool_result)\b[^>]*>([\s\S]*?)</\1>',
    caseSensitive: false,
  );
  var cursor = 0;
  for (final match in pattern.allMatches(content)) {
    if (match.start > cursor) {
      _addTextSegment(segments, content.substring(cursor, match.start));
    }
    final tag = match.group(1)!.toLowerCase();
    final body = match.group(2)!.trim();
    final kind = switch (tag) {
      'think' || 'thinking' => _AiSegmentKind.thinking,
      'status' => _AiSegmentKind.status,
      _ => _AiSegmentKind.tool,
    };
    if (body.isNotEmpty) {
      segments.add(_AiSegment(kind: kind, text: body));
    }
    cursor = match.end;
  }
  if (cursor < content.length) {
    _addTextSegment(segments, content.substring(cursor));
  }
  return segments;
}

void _addTextSegment(List<_AiSegment> segments, String text) {
  final cleaned = text.trim();
  if (cleaned.isNotEmpty) {
    segments.add(_AiSegment(kind: _AiSegmentKind.text, text: cleaned));
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
