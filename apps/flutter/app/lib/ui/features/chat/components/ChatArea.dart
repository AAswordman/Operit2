// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import '../../../common/markdown/StreamMarkdownRenderer.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatLayoutMetrics.dart';
import 'ChatScrollNavigator.dart';
import 'style/cursor/CursorStyleChatMessage.dart';

const Duration _navigatorHideDelay = Duration(milliseconds: 1200);

class ChatArea extends StatefulWidget {
  const ChatArea({
    super.key,
    required this.messages,
    required this.isLoading,
    required this.errorMessage,
    required this.scrollController,
    required this.currentChatId,
    required this.autoScrollToBottom,
    required this.hasOlderDisplayHistory,
    required this.hasNewerDisplayHistory,
    required this.isLoadingDisplayWindow,
    required this.loadLocatorEntries,
    required this.onAutoScrollToBottomChanged,
    required this.onLoadOlderDisplayWindow,
    required this.onLoadNewerDisplayWindow,
    required this.onShowLatestDisplayWindow,
    required this.onToggleFavoriteMessage,
  });

  final List<ChatUiMessage> messages;
  final bool isLoading;
  final String? errorMessage;
  final ScrollController scrollController;
  final String? currentChatId;
  final bool autoScrollToBottom;
  final bool hasOlderDisplayHistory;
  final bool hasNewerDisplayHistory;
  final bool isLoadingDisplayWindow;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final Future<void> Function() onLoadOlderDisplayWindow;
  final Future<void> Function() onLoadNewerDisplayWindow;
  final Future<void> Function() onShowLatestDisplayWindow;
  final ToggleFavoriteMessage onToggleFavoriteMessage;

  @override
  State<ChatArea> createState() => _ChatAreaState();
}

class _ChatAreaState extends State<ChatArea> {
  final GlobalKey _viewportKey = GlobalKey();
  final Map<int, GlobalKey> _messageKeys = <int, GlobalKey>{};
  Map<int, ChatScrollMessageAnchor> _messageAnchors =
      const <int, ChatScrollMessageAnchor>{};
  Timer? _navigatorHideTimer;
  bool _showNavigatorChip = false;
  bool _userScrollSessionActive = false;
  double _viewportHeight = 0;

  @override
  Widget build(BuildContext context) {
    final showLoadingIndicator = _shouldShowLoadingIndicator();
    final itemCount =
        widget.messages.length +
        (widget.hasOlderDisplayHistory ? 1 : 0) +
        (widget.hasNewerDisplayHistory ? 1 : 0) +
        (showLoadingIndicator || widget.errorMessage != null ? 1 : 0);

    if (itemCount == 0) {
      return const _EmptyChatArea();
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        _viewportHeight = constraints.maxHeight;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            _collectMessageAnchors();
          }
        });
        return Stack(
          key: _viewportKey,
          children: <Widget>[
            NotificationListener<ScrollNotification>(
              onNotification: _handleScrollNotification,
              child: ListView.separated(
                controller: widget.scrollController,
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
                itemCount: itemCount,
                separatorBuilder: (context, index) {
                  return const SizedBox(height: 8);
                },
                itemBuilder: (context, index) {
                  late final Widget child;
                  final messageStartIndex = widget.hasOlderDisplayHistory
                      ? 1
                      : 0;
                  final messageEndIndex =
                      messageStartIndex + widget.messages.length;
                  if (widget.hasOlderDisplayHistory && index == 0) {
                    child = _DisplayWindowAction(
                      text: 'Load more history',
                      isLoading: widget.isLoadingDisplayWindow,
                      onTap: () {
                        widget.onAutoScrollToBottomChanged(false);
                        if (!widget.isLoadingDisplayWindow) {
                          widget.onLoadOlderDisplayWindow();
                        }
                      },
                    );
                  } else if (index >= messageStartIndex &&
                      index < messageEndIndex) {
                    final message = widget.messages[index - messageStartIndex];
                    child = CursorStyleChatMessage(
                      key: ValueKey<String>(message.stableKey),
                      message: message,
                      isStreaming: _isStreamingMessage(
                        index - messageStartIndex,
                      ),
                    );
                  } else if (widget.hasNewerDisplayHistory &&
                      index == messageEndIndex) {
                    child = _DisplayWindowAction(
                      text: 'Load newer history',
                      isLoading: widget.isLoadingDisplayWindow,
                      onTap: () {
                        if (!widget.isLoadingDisplayWindow) {
                          widget.onLoadNewerDisplayWindow();
                        }
                      },
                    );
                  } else if (widget.errorMessage != null) {
                    child = _StatusMessage(
                      text: widget.errorMessage!,
                      isError: true,
                    );
                  } else {
                    child = const Padding(
                      padding: EdgeInsets.only(left: 16, top: 2, bottom: 2),
                      child: StreamingCursor(),
                    );
                  }
                  return _ChatAreaContentColumn(
                    key: _rowKeyForIndex(
                      index,
                      messageStartIndex,
                      messageEndIndex,
                    ),
                    child: child,
                  );
                },
              ),
            ),
            ChatScrollNavigator(
              messages: widget.messages,
              currentChatId: widget.currentChatId,
              scrollController: widget.scrollController,
              messageAnchors: _messageAnchors,
              viewportHeight: _viewportHeight,
              autoScrollToBottom: widget.autoScrollToBottom,
              hasNewerDisplayHistory: widget.hasNewerDisplayHistory,
              loadLocatorEntries: widget.loadLocatorEntries,
              onRequestLatestMessages: widget.onShowLatestDisplayWindow,
              onAutoScrollToBottomChanged: widget.onAutoScrollToBottomChanged,
              onJumpToMessage: _jumpToMessageTimestamp,
              onToggleFavoriteMessage: widget.onToggleFavoriteMessage,
              onRequestScrollToBottom: _scrollToBottomFromNavigator,
              showNavigatorChip: _showNavigatorChip,
              onNavigatorChipHidden: () {
                setState(() {
                  _showNavigatorChip = false;
                  _userScrollSessionActive = false;
                });
              },
            ),
          ],
        );
      },
    );
  }

  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification is UserScrollNotification) {
      if (notification.direction != ScrollDirection.idle) {
        _userScrollSessionActive = true;
        if (!_showNavigatorChip) {
          setState(() {
            _showNavigatorChip = true;
          });
        }
        if (notification.direction == ScrollDirection.forward &&
            widget.autoScrollToBottom &&
            !_isAtBottom(notification.metrics)) {
          widget.onAutoScrollToBottomChanged(false);
        }
      } else if (_userScrollSessionActive) {
        _scheduleNavigatorHide();
      }
    }

    if (notification is ScrollUpdateNotification) {
      if (!_showNavigatorChip) {
        _userScrollSessionActive = true;
        setState(() {
          _showNavigatorChip = true;
        });
      }
      _scheduleNavigatorHide();
      _collectMessageAnchors();
      if (_isAtBottom(notification.metrics) && !widget.autoScrollToBottom) {
        widget.onAutoScrollToBottomChanged(true);
      }
    }
    return false;
  }

  void _scheduleNavigatorHide() {
    _navigatorHideTimer?.cancel();
    _navigatorHideTimer = Timer(_navigatorHideDelay, () {
      if (!mounted ||
          widget.scrollController.position.isScrollingNotifier.value) {
        _scheduleNavigatorHide();
        return;
      }
      setState(() {
        _showNavigatorChip = false;
        _userScrollSessionActive = false;
      });
    });
  }

  bool _isAtBottom(ScrollMetrics metrics) {
    return metrics.pixels >= metrics.maxScrollExtent - 2;
  }

  Future<void> _scrollToBottomFromNavigator() async {
    widget.onAutoScrollToBottomChanged(true);
    if (widget.hasNewerDisplayHistory) {
      await widget.onShowLatestDisplayWindow();
      return;
    }
    await widget.scrollController.animateTo(
      widget.scrollController.position.maxScrollExtent,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
    );
  }

  void _jumpToMessageTimestamp(int timestamp) {
    final anchor = _messageAnchors[timestamp];
    if (anchor == null) {
      return;
    }
    final context = anchor.key.currentContext;
    if (context == null) {
      return;
    }
    Scrollable.ensureVisible(
      context,
      duration: const Duration(milliseconds: 260),
      curve: Curves.easeOutCubic,
      alignment: 0.1,
    );
  }

  void _collectMessageAnchors() {
    if (!widget.scrollController.hasClients) {
      return;
    }
    final viewportContext = _viewportKey.currentContext;
    final viewportBox = viewportContext?.findRenderObject() as RenderBox?;
    if (viewportBox == null) {
      return;
    }
    final anchors = <int, ChatScrollMessageAnchor>{};
    for (var index = 0; index < widget.messages.length; index++) {
      final message = widget.messages[index];
      final key = _keyForMessage(message.timestamp);
      final rowContext = key.currentContext;
      final rowBox = rowContext?.findRenderObject() as RenderBox?;
      if (rowBox == null || !rowBox.hasSize) {
        continue;
      }
      final localTop = rowBox
          .localToGlobal(Offset.zero, ancestor: viewportBox)
          .dy;
      anchors[message.timestamp] = ChatScrollMessageAnchor(
        timestamp: message.timestamp,
        index: index,
        key: key,
        absoluteTopPx: widget.scrollController.offset + localTop,
        heightPx: rowBox.size.height,
      );
    }
    if (anchors.length != _messageAnchors.length ||
        anchors.keys.any((key) => !_messageAnchors.containsKey(key))) {
      setState(() {
        _messageAnchors = anchors;
      });
    } else {
      _messageAnchors = anchors;
    }
  }

  GlobalKey _keyForMessage(int timestamp) {
    return _messageKeys.putIfAbsent(timestamp, GlobalKey.new);
  }

  Key _rowKeyForIndex(int index, int messageStartIndex, int messageEndIndex) {
    if (index >= messageStartIndex && index < messageEndIndex) {
      return _keyForMessage(
        widget.messages[index - messageStartIndex].timestamp,
      );
    }
    if (widget.hasOlderDisplayHistory && index == 0) {
      return const ValueKey<String>('row-load-older');
    }
    if (widget.hasNewerDisplayHistory && index == messageEndIndex) {
      return const ValueKey<String>('row-load-newer');
    }
    return const ValueKey<String>('row-status');
  }

  @override
  void didUpdateWidget(ChatArea oldWidget) {
    super.didUpdateWidget(oldWidget);
    final timestamps = widget.messages
        .map((message) => message.timestamp)
        .toSet();
    _messageKeys.removeWhere(
      (timestamp, key) => !timestamps.contains(timestamp),
    );
  }

  @override
  void dispose() {
    _navigatorHideTimer?.cancel();
    _messageKeys.clear();
    super.dispose();
  }

  bool _shouldShowLoadingIndicator() {
    if (!widget.isLoading || widget.messages.isEmpty) {
      return widget.isLoading && widget.messages.isEmpty;
    }
    final lastMessage = widget.messages.last;
    return lastMessage.sender == 'user' ||
        (lastMessage.sender == 'ai' && lastMessage.content.isEmpty);
  }

  bool _isStreamingMessage(int index) {
    if (!widget.isLoading || index < 0 || index >= widget.messages.length) {
      return false;
    }
    if (widget.messages[index].sender != 'ai') {
      return false;
    }
    for (var i = widget.messages.length - 1; i >= 0; i--) {
      if (widget.messages[i].sender == 'ai') {
        return i == index;
      }
    }
    return false;
  }
}

class _ChatAreaContentColumn extends StatelessWidget {
  const _ChatAreaContentColumn({super.key, required this.child});

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

class _DisplayWindowAction extends StatelessWidget {
  const _DisplayWindowAction({
    required this.text,
    required this.isLoading,
    required this.onTap,
  });

  final String text;
  final bool isLoading;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 16),
        child: Center(
          child: Text(
            isLoading ? 'Loading...' : text,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: theme.colorScheme.primary,
            ),
          ),
        ),
      ),
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
