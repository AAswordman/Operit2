// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../common/OperitLogoMark.dart';

class ChatToastHost extends StatefulWidget {
  const ChatToastHost({
    super.key,
    required this.message,
    required this.onDismiss,
    this.maxWidth = 720,
    this.maxHeight = 240,
  });

  final String? message;
  final VoidCallback onDismiss;
  final double maxWidth;
  final double maxHeight;

  @override
  State<ChatToastHost> createState() => _ChatToastHostState();
}

class _ChatToastHostState extends State<ChatToastHost> {
  final ScrollController _scrollController = ScrollController();
  Timer? _timer;

  @override
  void didUpdateWidget(ChatToastHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.message != widget.message) {
      _timer?.cancel();
      final message = widget.message;
      if (message == null || message.trim().isEmpty) {
        return;
      }
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(0);
      }
      _timer = Timer(_estimateDuration(message), widget.onDismiss);
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final message = widget.message;
    final visible = message != null && message.trim().isNotEmpty;
    final colorScheme = Theme.of(context).colorScheme;
    final estimatedLines = _estimatedLines(message ?? '');
    final compact = estimatedLines <= 2;

    return AnimatedSlide(
      offset: visible ? Offset.zero : const Offset(0, -0.35),
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      child: AnimatedOpacity(
        opacity: visible ? 1 : 0,
        duration: Duration(milliseconds: visible ? 180 : 140),
        child: IgnorePointer(
          ignoring: !visible,
          child: ConstrainedBox(
            constraints: BoxConstraints(maxWidth: widget.maxWidth),
            child: Material(
              color: colorScheme.surface.withValues(alpha: 0.98),
              elevation: 8,
              shadowColor: colorScheme.shadow.withValues(alpha: 0.18),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(8),
                side: BorderSide(color: colorScheme.outlineVariant),
              ),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(12, 8, 4, 8),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: <Widget>[
                    const OperitLogoMark(size: 36),
                    const SizedBox(width: 8),
                    Expanded(
                      child: ConstrainedBox(
                        constraints: BoxConstraints(
                          minHeight: 28,
                          maxHeight: compact
                              ? double.infinity
                              : widget.maxHeight,
                        ),
                        child: SingleChildScrollView(
                          controller: _scrollController,
                          physics: compact
                              ? const NeverScrollableScrollPhysics()
                              : const BouncingScrollPhysics(),
                          child: Align(
                            alignment: compact
                                ? Alignment.centerLeft
                                : Alignment.topLeft,
                            child: Text(
                              message ?? '',
                              style: Theme.of(context).textTheme.bodyMedium,
                              overflow: TextOverflow.clip,
                            ),
                          ),
                        ),
                      ),
                    ),
                    IconButton(
                      onPressed: widget.onDismiss,
                      icon: const Icon(Icons.close),
                      iconSize: 18,
                      constraints: const BoxConstraints.tightFor(
                        width: 28,
                        height: 28,
                      ),
                      padding: EdgeInsets.zero,
                      tooltip: MaterialLocalizations.of(
                        context,
                      ).closeButtonTooltip,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

int _estimatedLines(String message) {
  if (message.isEmpty) {
    return 1;
  }
  return message
      .split('\n')
      .map((line) => math.max(1, ((line.length + 23) / 24).floor()))
      .fold<int>(0, (sum, value) => sum + value);
}

Duration _estimateDuration(String message) {
  final milliseconds = 2500 + _estimatedLines(message) * 850;
  return Duration(milliseconds: milliseconds.clamp(3500, 12000));
}
