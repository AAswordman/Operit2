// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../viewmodel/ChatViewModel.dart';

typedef LoadMessageLocatorEntries =
    Future<List<ChatMessageLocatorPreview>> Function(
      String chatId,
      String query,
    );
typedef ToggleFavoriteMessage =
    Future<void> Function(int timestamp, bool isFavorite);
typedef RevealMessageForLocator = Future<bool> Function(int timestamp);

class ChatScrollMessageAnchor {
  const ChatScrollMessageAnchor({
    required this.timestamp,
    required this.index,
    required this.key,
    required this.absoluteTopPx,
    required this.heightPx,
  });

  final int timestamp;
  final int index;
  final GlobalKey key;
  final double absoluteTopPx;
  final double heightPx;
}

class ChatScrollNavigator extends StatefulWidget {
  const ChatScrollNavigator({
    super.key,
    required this.messages,
    required this.currentChatId,
    required this.scrollController,
    required this.messageAnchors,
    required this.viewportHeight,
    required this.autoScrollToBottom,
    required this.hasNewerDisplayHistory,
    required this.loadLocatorEntries,
    required this.onRequestLatestMessages,
    required this.onAutoScrollToBottomChanged,
    required this.onJumpToMessageTimestamp,
    required this.onJumpToMessage,
    required this.onToggleFavoriteMessage,
    required this.onRequestScrollToBottom,
    required this.showNavigatorChip,
    required this.onNavigatorChipHidden,
  });

  final List<ChatUiMessage> messages;
  final String? currentChatId;
  final ScrollController scrollController;
  final Map<int, ChatScrollMessageAnchor> messageAnchors;
  final double viewportHeight;
  final bool autoScrollToBottom;
  final bool hasNewerDisplayHistory;
  final LoadMessageLocatorEntries loadLocatorEntries;
  final Future<void> Function() onRequestLatestMessages;
  final ValueChanged<bool> onAutoScrollToBottomChanged;
  final ValueChanged<int> onJumpToMessageTimestamp;
  final ValueChanged<int> onJumpToMessage;
  final ToggleFavoriteMessage onToggleFavoriteMessage;
  final VoidCallback onRequestScrollToBottom;
  final bool showNavigatorChip;
  final VoidCallback onNavigatorChipHidden;

  @override
  State<ChatScrollNavigator> createState() => _ChatScrollNavigatorState();
}

class _ChatScrollNavigatorState extends State<ChatScrollNavigator> {
  List<ChatMessageLocatorPreview> _locatorEntries =
      const <ChatMessageLocatorPreview>[];

  int? _hoveredTimestamp;
  int? _hoveredTickIndex;

  @override
  void initState() {
    super.initState();
    _loadLocatorEntries();
  }

  @override
  void didUpdateWidget(ChatScrollNavigator oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.viewportHeight != widget.viewportHeight) {
      _hoveredTimestamp = null;
      _hoveredTickIndex = null;
    }
    final messagesChanged =
        oldWidget.messages.length != widget.messages.length ||
        oldWidget.messages.firstOrNull?.timestamp !=
            widget.messages.firstOrNull?.timestamp ||
        oldWidget.messages.lastOrNull?.timestamp !=
            widget.messages.lastOrNull?.timestamp;
    if (messagesChanged || oldWidget.currentChatId != widget.currentChatId) {
      _loadLocatorEntries();
    }
  }

  @override
  Widget build(BuildContext context) {
    if (widget.messages.isEmpty ||
        widget.viewportHeight <= 60.0 ||
        !widget.viewportHeight.isFinite) {
      return const SizedBox.shrink();
    }

    final entries = _getEntries();
    final activeTimestamp = _resolveActiveTimestamp();

    return Stack(
      children: <Widget>[
        // The Pure Codex Tick Rail (Vertically Centered, No Container, No Border)
        Positioned(
          right: 8,
          top: 0,
          bottom: 0,
          width: 24,
          child: Center(
            child: _CodexTicksColumn(
              entries: entries,
              activeTimestamp: activeTimestamp,
              viewportHeight: widget.viewportHeight,
              onHoverTick: (timestamp, index) {
                setState(() {
                  _hoveredTimestamp = timestamp;
                  _hoveredTickIndex = index;
                });
              },
              onTapTick: _handleJumpToMessage,
            ),
          ),
        ),

        // Hover preview tooltip (floats to the left of the hovered tick)
        if (_hoveredTimestamp != null && _hoveredTickIndex != null)
          Positioned(
            right: 36,
            top: _calculateTooltipTop(_hoveredTickIndex!, entries.length),
            child: _CodexHoverTooltipCard(
              entry: entries.firstWhere(
                (e) => e.timestamp == _hoveredTimestamp,
                orElse: () => _fallbackEntryFromTimestamp(_hoveredTimestamp!),
              ),
            ),
          ),

        // Subtle Floating "Scroll to Bottom" circle button (only when not at bottom, horizontally centered)
        if (!widget.autoScrollToBottom)
          Positioned(
            left: 0,
            right: 0,
            bottom: 16,
            child: Center(
              child: _ScrollToBottomCircleButton(
                onTap: widget.onRequestScrollToBottom,
              ),
            ),
          ),
      ],
    );
  }

  double _calculateTooltipTop(int index, int count) {
    if (count <= 0 || !widget.viewportHeight.isFinite) {
      return 16.0;
    }
    final maxAvailableHeight = math.max(30.0, widget.viewportHeight - 100.0);
    final step = count <= 25 ? 8.0 : (maxAvailableHeight / count).clamp(2.5, 8.0);
    final columnHeight = step * count;
    final startY = (widget.viewportHeight - columnHeight) / 2;
    final tickY = startY + (index + 0.5) * step;

    const minTop = 16.0;
    final maxTop = math.max(minTop, widget.viewportHeight - 90.0);
    return (tickY - 32.0).clamp(minTop, maxTop);
  }

  int? _resolveActiveTimestamp() {
    final scroll = widget.scrollController;
    if (!scroll.hasClients ||
        scroll.positions.length != 1 ||
        !scroll.position.hasPixels ||
        widget.viewportHeight <= 0 ||
        !widget.viewportHeight.isFinite ||
        widget.messageAnchors.isEmpty) {
      return widget.messages.lastOrNull?.timestamp;
    }
    final viewportCenter = scroll.offset + widget.viewportHeight / 2;
    ChatScrollMessageAnchor? centeredAnchor;
    double? centeredDistance;
    for (final anchor in widget.messageAnchors.values) {
      final distance =
          ((anchor.absoluteTopPx + anchor.heightPx / 2) - viewportCenter).abs();
      if (centeredDistance == null || distance < centeredDistance) {
        centeredAnchor = anchor;
        centeredDistance = distance;
      }
    }
    return centeredAnchor?.timestamp ?? widget.messages.lastOrNull?.timestamp;
  }

  List<ChatMessageLocatorPreview> _getEntries() {
    return _locatorEntries.isNotEmpty
        ? _locatorEntries
        : _fallbackEntriesFromMessages();
  }

  List<ChatMessageLocatorPreview> _fallbackEntriesFromMessages() {
    return widget.messages.indexed.map<ChatMessageLocatorPreview>((item) {
      final idx = item.$1;
      final msg = item.$2;
      final text = msg.displayText;
      return ChatMessageLocatorPreview(
        messageIndex: idx,
        timestamp: msg.timestamp,
        sender: msg.sender,
        previewContent: text,
        contentLength: text.length,
        displayMode: 'NORMAL',
        isFavorite: false,
      );
    }).toList(growable: false);
  }

  ChatMessageLocatorPreview _fallbackEntryFromTimestamp(int timestamp) {
    final msg = widget.messages.firstWhere(
      (m) => m.timestamp == timestamp,
      orElse: () => widget.messages.last,
    );
    final idx = widget.messages.indexOf(msg);
    final text = msg.displayText;
    return ChatMessageLocatorPreview(
      messageIndex: idx >= 0 ? idx : 0,
      timestamp: msg.timestamp,
      sender: msg.sender,
      previewContent: text,
      contentLength: text.length,
      displayMode: 'NORMAL',
      isFavorite: false,
    );
  }

  void _handleJumpToMessage(int timestamp) {
    final targetIndex = widget.messages.indexWhere(
      (m) => m.timestamp == timestamp,
    );
    if (targetIndex >= 0) {
      widget.onJumpToMessage(targetIndex);
    } else {
      widget.onJumpToMessageTimestamp(timestamp);
    }
  }

  Future<void> _loadLocatorEntries() async {
    final chatId = widget.currentChatId;
    if (chatId == null || chatId.trim().isEmpty) {
      setState(() {
        _locatorEntries = const <ChatMessageLocatorPreview>[];
      });
      return;
    }
    final entries = await widget.loadLocatorEntries(chatId, '');
    if (!mounted) {
      return;
    }
    setState(() {
      _locatorEntries = entries;
    });
  }
}

class _CodexTicksColumn extends StatefulWidget {
  const _CodexTicksColumn({
    required this.entries,
    required this.activeTimestamp,
    required this.viewportHeight,
    required this.onHoverTick,
    required this.onTapTick,
  });

  final List<ChatMessageLocatorPreview> entries;
  final int? activeTimestamp;
  final double viewportHeight;
  final void Function(int? timestamp, int? index) onHoverTick;
  final ValueChanged<int> onTapTick;

  @override
  State<_CodexTicksColumn> createState() => _CodexTicksColumnState();
}

class _CodexTicksColumnState extends State<_CodexTicksColumn> {
  int? _lastDragTickIndex;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final count = widget.entries.length;
    if (count == 0 || !widget.viewportHeight.isFinite) return const SizedBox.shrink();

    // Natural adaptive row height:
    // When few messages (e.g. 4), each item is 8px apart (32px total height).
    // When many messages, clamp down to 2.5px min.
    final maxAvailableHeight = math.max(30.0, widget.viewportHeight - 100.0);
    final step = count <= 25
        ? 8.0
        : (maxAvailableHeight / count).clamp(2.5, 8.0);

    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapDown: (details) {
        final idx = (details.localPosition.dy / step).floor().clamp(0, count - 1);
        widget.onTapTick(widget.entries[idx].timestamp);
      },
      onVerticalDragStart: (details) {
        final idx = (details.localPosition.dy / step).floor().clamp(0, count - 1);
        _lastDragTickIndex = idx;
        widget.onTapTick(widget.entries[idx].timestamp);
        widget.onHoverTick(widget.entries[idx].timestamp, idx);
      },
      onVerticalDragUpdate: (details) {
        final idx = (details.localPosition.dy / step).floor().clamp(0, count - 1);
        if (_lastDragTickIndex != idx) {
          _lastDragTickIndex = idx;
          widget.onTapTick(widget.entries[idx].timestamp);
          widget.onHoverTick(widget.entries[idx].timestamp, idx);
        }
      },
      onVerticalDragEnd: (_) {
        _lastDragTickIndex = null;
        widget.onHoverTick(null, null);
      },
      onVerticalDragCancel: () {
        _lastDragTickIndex = null;
        widget.onHoverTick(null, null);
      },
      child: MouseRegion(
        onExit: (_) => widget.onHoverTick(null, null),
        child: SingleChildScrollView(
          physics: const NeverScrollableScrollPhysics(),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            mainAxisAlignment: MainAxisAlignment.center,
            children: List<Widget>.generate(count, (index) {
              final entry = widget.entries[index];
              final isCurrent = entry.timestamp == widget.activeTimestamp;
              final isUser = entry.sender == 'user';
              final isFav = entry.isFavorite;

              final dashWidth = isCurrent ? 15.0 : (isUser ? 10.0 : 6.5);
              final dashHeight = isCurrent ? 2.8 : 1.8;
              final dashColor = isCurrent
                  ? theme.colorScheme.primary
                  : (isUser
                      ? theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.55)
                      : theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.28));

              return MouseRegion(
                cursor: SystemMouseCursors.click,
                onEnter: (_) {
                  widget.onHoverTick(entry.timestamp, index);
                },
                child: SizedBox(
                  height: step,
                  width: 24,
                  child: Center(
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Container(
                          width: dashWidth,
                          height: dashHeight,
                          decoration: BoxDecoration(
                            color: dashColor,
                            borderRadius: BorderRadius.circular(1.0),
                          ),
                        ),
                        if (isFav) ...[
                          const SizedBox(width: 2),
                          Container(
                            width: 2.2,
                            height: 2.2,
                            decoration: const BoxDecoration(
                              color: Colors.amber,
                              shape: BoxShape.circle,
                            ),
                          ),
                        ],
                      ],
                    ),
                  ),
                ),
              );
            }),
          ),
        ),
      ),
    );
  }
}

class _CodexHoverTooltipCard extends StatelessWidget {
  const _CodexHoverTooltipCard({required this.entry});

  final ChatMessageLocatorPreview entry;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final isUser = entry.sender == 'user';
    final previewText = buildMessagePreview(l10n, entry, '');

    return Material(
      color: Colors.transparent,
      child: Container(
        width: 210,
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHigh.withValues(alpha: 0.95),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
          ),
          boxShadow: [
            BoxShadow(
              color: Colors.black.withValues(alpha: 0.16),
              blurRadius: 10,
              offset: const Offset(0, 3),
            ),
          ],
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  isUser
                      ? Icons.person_rounded
                      : Icons.auto_awesome_rounded,
                  size: 11,
                  color: isUser
                      ? theme.colorScheme.primary
                      : theme.colorScheme.tertiary,
                ),
                const SizedBox(width: 4),
                Text(
                  isUser
                      ? '提问 #${(entry.messageIndex ?? 0) + 1}'
                      : senderLabel(context, entry.sender),
                  style: theme.textTheme.labelSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                    fontSize: 10,
                    color: isUser
                        ? theme.colorScheme.primary
                        : theme.colorScheme.onSurfaceVariant,
                  ),
                ),
                if (entry.isFavorite) ...[
                  const SizedBox(width: 4),
                  const Icon(Icons.star, size: 11, color: Colors.amber),
                ],
                const Spacer(),
                Text(
                  '点击跳转',
                  style: theme.textTheme.labelSmall?.copyWith(
                    fontSize: 9,
                    color: theme.colorScheme.primary,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 3),
            Text(
              previewText,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodySmall?.copyWith(
                fontSize: 11,
                color: theme.colorScheme.onSurface,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ScrollToBottomCircleButton extends StatelessWidget {
  const _ScrollToBottomCircleButton({required this.onTap});

  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: Colors.transparent,
      child: Tooltip(
        message: '回到底部',
        child: InkWell(
          borderRadius: BorderRadius.circular(14),
          onTap: onTap,
          child: Container(
            width: 28,
            height: 28,
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest.withValues(
                alpha: 0.75,
              ),
              shape: BoxShape.circle,
              border: Border.all(
                color: theme.colorScheme.outlineVariant.withValues(alpha: 0.22),
              ),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.12),
                  blurRadius: 6,
                  offset: const Offset(0, 2),
                ),
              ],
            ),
            child: Icon(
              Icons.keyboard_arrow_down_rounded,
              size: 16,
              color: theme.colorScheme.primary,
            ),
          ),
        ),
      ),
    );
  }
}

String senderLabel(BuildContext context, String sender) {
  final l10n = AppLocalizations.of(context)!;
  switch (sender) {
    case 'user':
      return l10n.messageSenderUser;
    case 'ai':
      return 'AI';
    case 'summary':
      return l10n.messageSenderSummary;
    case 'system':
      return l10n.messageSenderSystem;
    case 'think':
      return l10n.messageSenderThinking;
    default:
      return l10n.messageSenderOther;
  }
}

String visibleLocatorContent(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.sender == 'user' && preview.displayMode == 'HIDDEN_PLACEHOLDER') {
    return l10n.hiddenUserMessage;
  }
  return preview.previewContent;
}

int messageContentLength(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.contentLength > 0) {
    return preview.contentLength;
  }
  return math.max(visibleLocatorContent(l10n, preview).length, 1);
}

String buildMessagePreview(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
  String searchQuery,
) {
  final content = normalizeMessageSearchText(
    visibleLocatorContent(l10n, preview),
  );
  if (content.isEmpty) {
    return preview.sender;
  }
  final normalizedSearchQuery = normalizeMessageSearchText(searchQuery);
  if (normalizedSearchQuery.isNotEmpty) {
    final matchIndex = content.toLowerCase().indexOf(
      normalizedSearchQuery.toLowerCase(),
    );
    if (matchIndex >= 0) {
      const previewLength = 72;
      final preferredStart = math.max(matchIndex - 18, 0);
      final start = math.min(
        preferredStart,
        math.max(content.length - previewLength, 0),
      );
      final end = math.min(start + previewLength, content.length);
      final prefix = start > 0 ? '...' : '';
      final suffix = end < content.length ? '...' : '';
      final snippet = content.substring(start, end).trim();
      if (snippet.isNotEmpty) {
        return '$prefix$snippet$suffix';
      }
    }
  }
  final previewText = content.length > 72 ? content.substring(0, 72) : content;
  return previewText.length < content.length
      ? '${previewText.trimRight()}...'
      : previewText;
}

String normalizeMessageSearchText(String text) {
  if (text.isEmpty) {
    return '';
  }
  final buffer = StringBuffer();
  var pendingWhitespace = false;
  for (final codePoint in text.runes) {
    final char = String.fromCharCode(codePoint);
    final normalizedChar = char == '\n' || char == '\r' || char == '\t'
        ? ' '
        : char;
    if (normalizedChar.trim().isEmpty) {
      pendingWhitespace = buffer.isNotEmpty;
      continue;
    }
    if (pendingWhitespace) {
      buffer.write(' ');
      pendingWhitespace = false;
    }
    buffer.write(normalizedChar);
  }
  return buffer.toString().trim();
}
