// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/chat/OperitChatRuntime.dart';
import 'ChatLayoutMetrics.dart';
import 'style/cursor/CursorStyleChatMessage.dart';

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
        late final Widget child;
        if (index < messages.length) {
          child = CursorStyleChatMessage(
            message: messages[index],
            isStreaming: _isStreamingMessage(index),
          );
        } else if (errorMessage != null) {
          child = _StatusMessage(text: errorMessage!, isError: true);
        } else {
          child = const Padding(
            padding: EdgeInsets.only(left: 16, top: 2, bottom: 2),
            child: LoadingDotsIndicator(),
          );
        }
        return _ChatAreaContentColumn(child: child);
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

  bool _isStreamingMessage(int index) {
    if (!isLoading || index < 0 || index >= messages.length) {
      return false;
    }
    if (messages[index].sender != 'ai') {
      return false;
    }
    for (var i = messages.length - 1; i >= 0; i--) {
      if (messages[i].sender == 'ai') {
        return i == index;
      }
    }
    return false;
  }
}

class _ChatAreaContentColumn extends StatelessWidget {
  const _ChatAreaContentColumn({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: chatContentMaxWidth),
        child: SizedBox(width: double.infinity, child: child),
      ),
    );
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
